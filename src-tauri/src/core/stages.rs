// ============================================================================
// Pipeline Stages
// ============================================================================
// Individual processing stages:
// - Probing: validate file with ffprobe
// - Extracting: convert video to opus audio via ffmpeg
// - Uploading: upload audio to Groq API
// - Transcribing: wait for Groq to transcribe
// - Stitching: (optional) merge chunks if chunking was needed
// - Postprocessing: (optional) clean up via Groq Llama
// - Writing: save transcript to .txt

use crate::adapters::ffmpeg::FfmpegAdapter;
use crate::adapters::groq::{GroqClient, TranscribeOpts};
use crate::types::{AppErrorView, Job, JobState, TranscriptSegment};
use std::path::PathBuf;

/// Context for a job during pipeline execution
pub struct PipelineCtx {
    pub job: Job,
    /// Temporary audio file (opus)
    pub audio_temp_path: Option<PathBuf>,
    /// Segments from transcription
    pub segments: Vec<TranscriptSegment>,
    /// Final transcript text
    pub transcript_text: String,
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
            segments: Vec::new(),
            transcript_text: String::new(),
            output_path,
        }
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

/// Upload stage: send audio to Groq API
pub async fn upload(ctx: &PipelineCtx, _groq: &GroqClient) -> Result<StageResult, AppErrorView> {
    let audio_path = ctx
        .audio_temp_path
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("Audio temp path not set"))?;

    // Get file size for progress tracking
    let _file_size = std::fs::metadata(audio_path)
        .map_err(|e| AppErrorView::fs_error(format!("Failed to stat audio file: {}", e)))?
        .len();

    // Transcribe (upload + transcribe in one call)
    // We'll update state to Uploading then Transcribing
    Ok(StageResult {
        state: JobState::Uploading {
            progress: 0.0,
            chunk_idx: 1,
            chunk_total: 1,
        },
        should_continue: true,
    })
}

/// Transcribe stage: wait for Groq transcription result
pub async fn transcribe(
    ctx: &mut PipelineCtx,
    groq: &GroqClient,
) -> Result<StageResult, AppErrorView> {
    let audio_path = ctx
        .audio_temp_path
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("Audio temp path not set"))?;

    let opts = TranscribeOpts {
        language: ctx.job.settings_snapshot.language.clone(),
        temperature: 0.0,
        prompt: "This is a recording in Russian. One person speaks.".to_string(),
        model: "whisper-large-v3-turbo".to_string(),
        response_format: "verbose_json".to_string(),
    };
    let result = groq.transcribe(audio_path, opts).await?;

    ctx.transcript_text = result.text;
    ctx.segments = result
        .segments
        .into_iter()
        .map(|seg| TranscriptSegment {
            start_ms: (seg.start * 1000.0) as u64,
            end_ms: (seg.end * 1000.0) as u64,
            text: seg.text,
        })
        .collect();

    Ok(StageResult {
        state: JobState::Transcribing {
            chunk_idx: 1,
            chunk_total: 1,
        },
        should_continue: true,
    })
}

/// Write stage: save transcript to .txt file atomically
pub async fn write_transcript(ctx: &PipelineCtx) -> Result<StageResult, AppErrorView> {
    // Write to temporary file first, then rename
    let output_dir = ctx
        .output_path
        .parent()
        .ok_or_else(|| AppErrorView::fs_error("Invalid output path"))?;

    let temp_path = output_dir.join(format!(
        ".{}.tmp",
        ctx.output_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("transcript")
    ));

    // Write to temp file
    std::fs::write(&temp_path, &ctx.transcript_text)
        .map_err(|e| AppErrorView::fs_error(format!("Failed to write temp transcript: {}", e)))?;

    // Atomic rename
    std::fs::rename(&temp_path, &ctx.output_path).map_err(|e| {
        // Clean up temp file on error
        let _ = std::fs::remove_file(&temp_path);
        AppErrorView::fs_error(format!("Failed to write transcript: {}", e))
    })?;

    Ok(StageResult {
        state: JobState::Done {
            output_path: ctx.output_path.clone(),
            duration_ms: (ctx.segments.last().map(|s| s.end_ms).unwrap_or(0)) as u64,
        },
        should_continue: false,
    })
}

/// Cleanup: remove temporary files
pub async fn cleanup(ctx: &mut PipelineCtx) {
    if let Some(audio_path) = &ctx.audio_temp_path {
        let _ = tokio::fs::remove_file(audio_path).await;
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
