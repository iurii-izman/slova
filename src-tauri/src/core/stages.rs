// ============================================================================
// Pipeline Stages
// ============================================================================
// Individual processing stages:
// - Probing: validate file with ffprobe
// - Extracting: convert video to opus audio via ffmpeg
// - Chunking: split audio if needed (file >100 MB)
// - Uploading: upload audio to Groq API
// - Transcribing: wait for Groq to transcribe
// - Stitching: (optional) merge chunks if chunking was needed
// - Writing: save transcript to .txt

use crate::adapters::ffmpeg::FfmpegAdapter;
use crate::adapters::groq::{GroqClient, TranscribeOpts};
use crate::core::chunking::{self, AudioChunk};
use crate::core::export::{export_transcript, write_atomic, ConflictPolicy};
use crate::core::stitching::{self, ChunkTranscript, SegmentLocal};
use crate::types::{AppErrorView, Job, JobState, TranscriptSegment};
use std::path::PathBuf;

/// Context for a job during pipeline execution
pub struct PipelineCtx {
    pub job: Job,
    /// Temporary audio file (opus)
    pub audio_temp_path: Option<PathBuf>,
    /// Audio chunks (if chunking was needed)
    pub chunks: Vec<AudioChunk>,
    /// Transcribed chunks (if chunking was needed)
    pub chunk_transcripts: Vec<ChunkTranscript>,
    /// Segments from transcription
    pub segments: Vec<TranscriptSegment>,
    /// Raw transcript text (from Groq Whisper)
    pub transcript_raw: String,
    /// Processed transcript text (after Llama postprocessing, if enabled)
    pub transcript_processed: Option<String>,
    /// Output path where transcript will be saved
    pub output_path: PathBuf,
}

impl PipelineCtx {
    pub fn new(job: Job) -> Self {
        // Determine output path (same directory as source, .txt)
        let output_path = job.source_path.with_extension("txt");

        PipelineCtx {
            job,
            audio_temp_path: None,
            chunks: Vec::new(),
            chunk_transcripts: Vec::new(),
            segments: Vec::new(),
            transcript_raw: String::new(),
            transcript_processed: None,
            output_path,
        }
    }
}

impl PipelineCtx {
    /// Get the final transcript to write (processed if available, otherwise raw)
    pub fn get_final_transcript(&self) -> &str {
        self.transcript_processed
            .as_deref()
            .unwrap_or(&self.transcript_raw)
    }
}

/// Stage result with optional next state
pub struct StageResult {
    pub state: JobState,
    pub should_continue: bool,
}

/// Probe stage: validate file structure with ffprobe
pub async fn probe(
    ctx: &mut PipelineCtx,
    ffmpeg: &FfmpegAdapter,
) -> Result<StageResult, AppErrorView> {
    let result = ffmpeg.probe(&ctx.job.source_path).await?;

    // Validate: file must have audio
    if !result.has_audio {
        return Err(AppErrorView::invalid_file("Video file has no audio track"));
    }

    // Validate: reasonable duration (at least 1 second)
    if result.duration_seconds < 1.0 {
        return Err(AppErrorView::invalid_file(
            "Audio duration too short (< 1 second)",
        ));
    }

    Ok(StageResult {
        state: JobState::Probing,
        should_continue: true,
    })
}

/// Extract stage: convert video to mono opus 16kHz
pub async fn extract(
    ctx: &mut PipelineCtx,
    ffmpeg: &FfmpegAdapter,
) -> Result<StageResult, AppErrorView> {
    let temp_audio = tempfile::NamedTempFile::new()
        .map_err(|e| AppErrorView::fs_error(format!("Failed to create temp file: {}", e)))?
        .path()
        .to_path_buf();

    // Get duration from probe for progress tracking
    let probe_result = ffmpeg.probe(&ctx.job.source_path).await?;
    let total_duration_ms = (probe_result.duration_seconds * 1000.0) as u64;

    // Extract audio to opus
    ffmpeg
        .extract_audio(&ctx.job.source_path, &temp_audio, total_duration_ms, None)
        .await?;

    ctx.audio_temp_path = Some(temp_audio);

    Ok(StageResult {
        state: JobState::Extracting { progress: 1.0 },
        should_continue: true,
    })
}

/// Chunking stage: split audio if needed (file >100 MB)
pub async fn chunk(
    ctx: &mut PipelineCtx,
    ffmpeg: &FfmpegAdapter,
) -> Result<StageResult, AppErrorView> {
    let audio_path = ctx
        .audio_temp_path
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("Audio temp path not set"))?
        .clone();

    let audio_size = std::fs::metadata(&audio_path)
        .map_err(|e| AppErrorView::fs_error(format!("Failed to stat audio file: {}", e)))?
        .len();

    // Get probe info for duration
    let probe_result = ffmpeg.probe(&ctx.job.source_path).await?;
    let total_duration_ms = (probe_result.duration_seconds * 1000.0) as u64;

    // Check if chunking is needed
    if !chunking::should_chunk(audio_size) {
        // No chunking needed, create a single "chunk" for the pipeline
        ctx.chunks = vec![AudioChunk {
            start_ms: 0,
            end_ms: total_duration_ms,
            overlap_start_ms: None,
            overlap_end_ms: None,
            path: audio_path,
            idx: 0,
            total: 1,
        }];

        return Ok(StageResult {
            state: JobState::Uploading {
                progress: 0.0,
                chunk_idx: 1,
                chunk_total: 1,
            },
            should_continue: true,
        });
    }

    // Chunking is needed
    // Detect silence points
    let silence_points = ffmpeg.silence_detect(&audio_path).await.unwrap_or_default();

    // Estimate bitrate
    let bitrate_kbps = chunking::estimate_bitrate_kbps(audio_size, total_duration_ms);

    // Calculate chunk boundaries
    let boundaries =
        chunking::calculate_chunk_boundaries(total_duration_ms, &silence_points, bitrate_kbps)?;

    // Add overlaps
    let boundaries_with_overlap = chunking::add_overlaps(boundaries, 5000); // 5 sec overlap

    let mut chunks = Vec::new();
    let total_chunks = boundaries_with_overlap.len() as u32;

    // Create chunk files
    for (idx, (start, end, overlap_start, overlap_end)) in
        boundaries_with_overlap.iter().enumerate()
    {
        let chunk_file = tempfile::NamedTempFile::new()
            .map_err(|e| {
                AppErrorView::fs_error(format!("Failed to create chunk temp file: {}", e))
            })?
            .path()
            .to_path_buf();

        // Cut the chunk using ffmpeg
        ffmpeg
            .cut(&audio_path, *start, end - start, &chunk_file)
            .await?;

        chunks.push(AudioChunk {
            start_ms: *start,
            end_ms: *end,
            overlap_start_ms: *overlap_start,
            overlap_end_ms: *overlap_end,
            path: chunk_file,
            idx: idx as u32,
            total: total_chunks,
        });
    }

    ctx.chunks = chunks;

    Ok(StageResult {
        state: JobState::Chunking { progress: 1.0 },
        should_continue: true,
    })
}

/// Transcribe stage: wait for Groq transcription result (handles single chunk or multiple chunks)
pub async fn transcribe(
    ctx: &mut PipelineCtx,
    groq: &GroqClient,
) -> Result<StageResult, AppErrorView> {
    if ctx.chunks.is_empty() {
        return Err(AppErrorView::internal_error(
            "No chunks available for transcription",
        ));
    }

    let total_chunks = ctx.chunks.len() as u32;
    let opts = TranscribeOpts {
        language: ctx.job.settings_snapshot.language.clone(),
        temperature: 0.0,
        prompt: "This is a recording in Russian. One person speaks.".to_string(),
        model: "whisper-large-v3-turbo".to_string(),
        response_format: "verbose_json".to_string(),
    };

    // Transcribe each chunk
    for (idx, chunk) in ctx.chunks.iter().enumerate() {
        let result = groq.transcribe(&chunk.path, opts.clone()).await?;

        ctx.chunk_transcripts.push(ChunkTranscript {
            chunk_idx: idx as u32,
            chunk_start_ms: chunk.start_ms,
            chunk_end_ms: chunk.end_ms,
            overlap_start_ms: chunk.overlap_start_ms,
            overlap_end_ms: chunk.overlap_end_ms,
            text: result.text,
            segments: result
                .segments
                .into_iter()
                .map(|seg| SegmentLocal {
                    start: seg.start,
                    end: seg.end,
                    text: seg.text,
                })
                .collect(),
        });
    }

    Ok(StageResult {
        state: JobState::Transcribing {
            chunk_idx: total_chunks,
            chunk_total: total_chunks,
        },
        should_continue: true,
    })
}

/// Stitching stage: merge chunk transcripts into final transcript
pub async fn stitch_transcript(ctx: &mut PipelineCtx) -> Result<StageResult, AppErrorView> {
    if ctx.chunk_transcripts.is_empty() {
        return Err(AppErrorView::internal_error(
            "No chunk transcripts available",
        ));
    }

    let (text, segments) = stitching::stitch_chunks(ctx.chunk_transcripts.clone())?;

    ctx.transcript_raw = text;
    ctx.segments = segments;

    Ok(StageResult {
        state: JobState::Stitching,
        should_continue: true,
    })
}

/// Postprocessing stage: clean transcript via Groq Llama (optional)
/// If enabled, stores processed version in transcript_processed
/// If postprocessing fails, logs warning but completes with raw transcript
pub async fn postprocess_transcript(
    ctx: &mut PipelineCtx,
    groq: &GroqClient,
    postprocess_model: &str,
) -> Result<StageResult, AppErrorView> {
    // If postprocessing is disabled, skip
    if !ctx.job.settings_snapshot.enable_postprocess {
        return Ok(StageResult {
            state: JobState::Postprocessing,
            should_continue: true,
        });
    }

    match groq
        .postprocess(ctx.transcript_raw.clone(), postprocess_model)
        .await
    {
        Ok(processed) => {
            ctx.transcript_processed = Some(processed);
            Ok(StageResult {
                state: JobState::Postprocessing,
                should_continue: true,
            })
        }
        Err(err) => {
            // Log warning but don't fail the job
            eprintln!(
                "⚠️ Postprocessing failed for job {}: {}. Continuing with raw transcript.",
                ctx.job.id, err
            );
            // Set processed to None and continue with raw
            ctx.transcript_processed = None;
            Ok(StageResult {
                state: JobState::Postprocessing,
                should_continue: true,
            })
        }
    }
}

/// Write stage: save transcript to .txt file atomically
pub async fn write_transcript(ctx: &PipelineCtx) -> Result<StageResult, AppErrorView> {
    // Always save raw transcript first
    let _raw_path = write_atomic(
        &ctx.output_path,
        ctx.transcript_raw.as_bytes(),
        ConflictPolicy::Overwrite,
    )
    .await?;

    // If postprocessing was applied, save processed version with suffix
    if let Some(processed) = &ctx.transcript_processed {
        let output_stem = ctx
            .output_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("transcript");

        let processed_path = ctx
            .output_path
            .parent()
            .map(|p| p.join(format!("{}.processed.txt", output_stem)));

        if let Some(path) = processed_path {
            let _ = write_atomic(&path, processed.as_bytes(), ConflictPolicy::Overwrite).await;
        }
    }

    // Return the output path (main transcript, which is now raw)
    let final_path = ctx.output_path.clone();
    let duration_ms = ctx.segments.last().map(|s| s.end_ms).unwrap_or(0);

    Ok(StageResult {
        state: JobState::Done {
            output_path: final_path,
            duration_ms,
        },
        should_continue: false,
    })
}

/// Export transcript in specified format (SRT or JSON)
pub async fn export(ctx: &PipelineCtx, format: &str) -> Result<PathBuf, AppErrorView> {
    let base_path = ctx.output_path.with_extension("");

    export_transcript(
        ctx.get_final_transcript(),
        &ctx.segments,
        format,
        &base_path,
        ConflictPolicy::Overwrite,
    )
    .await
}

/// Cleanup: remove temporary files
pub async fn cleanup(ctx: &mut PipelineCtx) {
    if let Some(audio_path) = &ctx.audio_temp_path {
        let _ = tokio::fs::remove_file(audio_path).await;
    }
    // Cleanup chunk files
    for chunk in &ctx.chunks {
        let _ = tokio::fs::remove_file(&chunk.path).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_ctx_creation() {
        let job = Job {
            id: Default::default(),
            source_path: PathBuf::from("/test/video.mp4"),
            display_name: "test".into(),
            size_bytes: 1000,
            created_at: "2024-01-01".into(),
            state: JobState::Queued,
            settings_snapshot: Default::default(),
            content_hash: None,
        };

        let ctx = PipelineCtx::new(job);
        assert_eq!(ctx.output_path.extension().unwrap(), "txt");
    }
}
