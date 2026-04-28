// ============================================================================
// FFmpeg Adapter
// ============================================================================
// Wrapper around ffmpeg/ffprobe sidecars for:
// - Video validation (ffprobe)
// - Audio extraction: MP4 → Opus 16kHz 32kbps
// - Silence detection for chunking
// - Noise reduction via rnnoise filter

#![allow(dead_code)] // FFmpeg adapter is used by core scheduler, but clippy doesn't see it yet

use crate::types::AppErrorView;
use serde::Deserialize;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

/// Typesafe wrapper around ffmpeg/ffprobe executables
pub struct FfmpegAdapter {
    /// Path to ffmpeg binary (bundled sidecar)
    pub ffmpeg_exe: PathBuf,
    /// Path to ffprobe binary
    pub ffprobe_exe: PathBuf,
    /// Path to rnnoise model for noise reduction
    pub rnnoise_model: PathBuf,
}

impl FfmpegAdapter {
    pub fn new(ffmpeg_exe: PathBuf, ffprobe_exe: PathBuf, rnnoise_model: PathBuf) -> Self {
        FfmpegAdapter {
            ffmpeg_exe,
            ffprobe_exe,
            rnnoise_model,
        }
    }

    /// Create with system ffmpeg/ffprobe (for development)
    pub fn default_new() -> Self {
        FfmpegAdapter {
            ffmpeg_exe: PathBuf::from("ffmpeg"),
            ffprobe_exe: PathBuf::from("ffprobe"),
            rnnoise_model: PathBuf::from("rnnoise_model"),
        }
    }

    /// Validate MP4 file and get metadata (duration, audio tracks, format, size)
    /// Uses ffprobe with JSON output: -v quiet -print_format json -show_format -show_streams
    pub async fn probe(&self, path: &Path) -> Result<ProbeResult, AppErrorView> {
        // Validate path exists
        if !path.exists() {
            return Err(AppErrorView::invalid_file("File not found"));
        }

        // Build ffprobe command: ffprobe -v quiet -print_format json -show_format -show_streams
        let output = Command::new(&self.ffprobe_exe)
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(path)
            .output()
            .await
            .map_err(|e| AppErrorView::internal_error(format!("Failed to run ffprobe: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppErrorView::invalid_file(format!(
                "ffprobe failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        let ffprobe_output: FfprobeOutput = serde_json::from_str(&stdout).map_err(|e| {
            AppErrorView::internal_error(format!("Failed to parse ffprobe JSON: {}", e))
        })?;

        // Extract format information
        let format = &ffprobe_output.format;
        let duration_seconds = format
            .duration
            .as_deref()
            .and_then(|d| d.parse::<f64>().ok())
            .ok_or_else(|| AppErrorView::invalid_file("Could not determine duration"))?;

        let file_size_bytes = format
            .size
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| AppErrorView::invalid_file("Could not determine file size"))?;

        // Check for audio streams
        let has_audio = ffprobe_output
            .streams
            .iter()
            .any(|s| s.codec_type.as_deref() == Some("audio"));

        if !has_audio {
            return Err(AppErrorView::invalid_file("File has no audio stream"));
        }

        // Get audio codec from first audio stream
        let audio_codec = ffprobe_output
            .streams
            .iter()
            .find(|s| s.codec_type.as_deref() == Some("audio"))
            .and_then(|s| s.codec_name.clone());

        Ok(ProbeResult {
            duration_seconds,
            has_audio,
            audio_codec,
            file_size_bytes,
            nb_streams: ffprobe_output.streams.len(),
        })
    }

    /// Extract audio from MP4 to Opus 16kHz 32kbps with optional noise reduction
    /// Command: ffmpeg -i input.mp4 -vn -ac 1 -ar 16000
    ///          -af "arnndn=m=rnnoise-models/cb.rnnn" -c:a libopus -b:a 32k output.opus
    ///
    /// Returns ExtractStats with output size and noise reduction status.
    /// Progress updates are written to the optional progress_tx channel if provided.
    pub async fn extract_audio(
        &self,
        input: &Path,
        output: &Path,
        total_duration_ms: u64,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<f32>>,
    ) -> Result<ExtractStats, AppErrorView> {
        // Check if rnnoise model exists; if not, warn and skip filter
        let has_rnnoise = self.rnnoise_model.exists();

        // Build audio filter chain
        let audio_filter = if has_rnnoise {
            format!("arnndn=m={}", self.rnnoise_model.display())
        } else {
            String::new()
        };

        let mut cmd = Command::new(&self.ffmpeg_exe);
        cmd.arg("-i")
            .arg(input)
            .arg("-vn") // no video
            .arg("-ac")
            .arg("1") // mono
            .arg("-ar")
            .arg("16000") // 16 kHz sample rate
            .arg("-c:a")
            .arg("libopus") // opus codec
            .arg("-b:a")
            .arg("32k"); // 32 kbps bitrate

        // Only add audio filter if we have a valid filter and model exists
        if !audio_filter.is_empty() {
            cmd.arg("-af").arg(&audio_filter);
        }

        // Progress output to stderr
        cmd.arg("-progress")
            .arg("pipe:2")
            .arg(output)
            .stdout(Stdio::null());

        // Capture stderr for progress parsing
        let mut child = cmd
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppErrorView::internal_error(format!("Failed to spawn ffmpeg: {}", e)))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppErrorView::internal_error("Could not capture ffmpeg stderr"))?;

        let mut reader = tokio::io::BufReader::new(stderr);
        let mut line = String::new();

        // Spawn progress reading task with progress_tx moved into closure
        let progress_handle = if let Some(tx) = progress_tx {
            tokio::spawn(async move {
                loop {
                    if reader.read_line(&mut line).await.is_err() || line.is_empty() {
                        break;
                    }

                    // Parse progress lines like: out_time_us=123456789
                    if line.starts_with("out_time_us=") {
                        if let Some(us) = line
                            .trim()
                            .strip_prefix("out_time_us=")
                            .and_then(|s| s.parse::<u64>().ok())
                        {
                            let ms = us / 1000;
                            let progress = if total_duration_ms > 0 {
                                (ms as f32 / total_duration_ms as f32).min(1.0)
                            } else {
                                0.0
                            };

                            // Send progress update
                            let _ = tx.send(progress);
                        }
                    }
                    line.clear();
                }
            })
        } else {
            tokio::spawn(async move {
                loop {
                    if reader.read_line(&mut line).await.is_err() || line.is_empty() {
                        break;
                    }
                    line.clear();
                }
            })
        };

        // Wait for ffmpeg to finish
        let status = child.wait().await.map_err(|e| {
            AppErrorView::internal_error(format!("Failed to wait for ffmpeg: {}", e))
        })?;

        // Wait for progress task to finish (with short timeout)
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), progress_handle).await;

        if !status.success() {
            // Try to get file size of partially-written output for error info
            let partial_size = std::fs::metadata(output).ok().map(|m| m.len()).unwrap_or(0);

            return Err(AppErrorView::internal_error(format!(
                "ffmpeg failed with exit code {:?} (bytes written: {})",
                status.code(),
                partial_size
            )));
        }

        // Get output file size
        let output_size = std::fs::metadata(output)
            .map_err(|e| AppErrorView::fs_error(format!("Failed to stat output file: {}", e)))?
            .len();

        // Ensure we got a valid output
        if output_size == 0 {
            return Err(AppErrorView::internal_error(
                "ffmpeg produced empty output file",
            ));
        }

        // Note: final progress update (1.0) was sent inside the progress_tx match
        // or will be inferred from successful completion

        Ok(ExtractStats {
            output_size_bytes: output_size,
            noise_reduction_applied: has_rnnoise,
        })
    }

    /// Detect silence points for chunking (fallback for >100MB files)
    /// Uses ffmpeg silencedetect filter with JSON output
    pub async fn silence_detect(
        &self,
        audio_path: &Path,
    ) -> Result<Vec<SilencePoint>, AppErrorView> {
        // Execute: ffmpeg -i input.opus -af silencedetect=n=-40dB:d=0.5 -f null -
        let output = Command::new(&self.ffmpeg_exe)
            .arg("-i")
            .arg(audio_path)
            .arg("-af")
            .arg("silencedetect=n=-40dB:d=0.5")
            .arg("-f")
            .arg("null")
            .arg("-")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to run ffmpeg silencedetect: {}", e))
            })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut silence_spans = Vec::new();

        // Parse silence_start and silence_end lines from stderr
        // Format: [silencedetect @ ...] silence_start: 0.123456
        // Format: [silencedetect @ ...] silence_end: 1.234567 | silence_duration: 1.111111
        let mut current_start: Option<f64> = None;

        for line in stderr.lines() {
            if line.contains("silence_start:") {
                if let Some(pos) = line.find("silence_start:") {
                    let after = line[pos + 14..].trim();
                    if let Ok(start) = after.parse::<f64>() {
                        current_start = Some(start);
                    }
                }
            } else if line.contains("silence_end:") {
                if let Some(pos) = line.find("silence_end:") {
                    let after = line[pos + 12..].trim();
                    if let Some(end) = after
                        .split('|')
                        .next()
                        .map(|s| s.trim())
                        .and_then(|s| s.parse::<f64>().ok())
                    {
                        if let Some(start) = current_start {
                            silence_spans.push(SilencePoint {
                                start_ms: (start * 1000.0) as u64,
                                end_ms: (end * 1000.0) as u64,
                            });
                        }
                    }
                }
            }
        }

        Ok(silence_spans)
    }

    /// Cut audio at specified time ranges
    /// Uses ffmpeg with ss (seek) and t (duration) options
    pub async fn cut(
        &self,
        audio_path: &Path,
        start_ms: u64,
        duration_ms: u64,
        output: &Path,
    ) -> Result<(), AppErrorView> {
        let start_secs = start_ms as f64 / 1000.0;
        let duration_secs = duration_ms as f64 / 1000.0;

        let status = Command::new(&self.ffmpeg_exe)
            .arg("-ss")
            .arg(format!("{:.3}", start_secs))
            .arg("-i")
            .arg(audio_path)
            .arg("-t")
            .arg(format!("{:.3}", duration_secs))
            .arg("-c")
            .arg("copy") // direct copy, no re-encoding
            .arg(output)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to run ffmpeg cut: {}", e))
            })?;

        if !status.success() {
            return Err(AppErrorView::internal_error(format!(
                "ffmpeg cut failed with exit code {:?}",
                status.code()
            )));
        }

        Ok(())
    }
}

// ============================================================================
// FFprobe JSON Schema
// ============================================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FfprobeOutput {
    format: FfprobeFormat,
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FfprobeFormat {
    duration: Option<String>,
    size: Option<String>,
    #[serde(default)]
    nb_streams: u32,
    #[serde(default)]
    format_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FfprobeStream {
    index: u32,
    codec_type: Option<String>,
    codec_name: Option<String>,
    #[serde(default)]
    duration: Option<String>,
}

// ============================================================================
// Result types
// ============================================================================

/// Result from ffprobe (metadata)
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub duration_seconds: f64,
    pub has_audio: bool,
    pub audio_codec: Option<String>,
    pub file_size_bytes: u64,
    pub nb_streams: usize,
}

/// Statistics from audio extraction
#[derive(Debug, Clone)]
pub struct ExtractStats {
    pub output_size_bytes: u64,
    pub noise_reduction_applied: bool,
}

/// Silence point detected by ffmpeg silencedetect
#[derive(Debug, Clone)]
pub struct SilencePoint {
    pub start_ms: u64,
    pub end_ms: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test parsing ffprobe JSON output
    #[test]
    fn test_parse_ffprobe_output() {
        let json_sample = r#"{"format":{"duration":"123.456","size":"1000000","nb_streams":2},"streams":[{"index":0,"codec_type":"video","codec_name":"h264"},{"index":1,"codec_type":"audio","codec_name":"aac"}]}"#;

        let output: FfprobeOutput =
            serde_json::from_str(json_sample).expect("Failed to parse ffprobe JSON");

        assert_eq!(output.format.duration.as_deref(), Some("123.456"));
        assert_eq!(output.format.size.as_deref(), Some("1000000"));
        assert_eq!(output.streams.len(), 2);

        let audio_stream = output
            .streams
            .iter()
            .find(|s| s.codec_type.as_deref() == Some("audio"))
            .expect("Should have audio stream");
        assert_eq!(audio_stream.codec_name.as_deref(), Some("aac"));
    }

    /// Test parsing ffprobe with missing optional fields
    #[test]
    fn test_parse_ffprobe_minimal() {
        let json_minimal =
            r#"{"format":{"duration":"30.5"},"streams":[{"index":0,"codec_type":"audio"}]}"#;

        let output: FfprobeOutput =
            serde_json::from_str(json_minimal).expect("Failed to parse minimal ffprobe JSON");

        assert_eq!(output.format.duration.as_deref(), Some("30.5"));
        assert!(output.format.size.is_none());
        assert_eq!(output.streams.len(), 1);
    }

    /// Test parsing progress output from ffmpeg
    #[test]
    fn test_parse_progress_output() {
        let progress_line = "out_time_us=1234567890";
        let us_str = progress_line
            .strip_prefix("out_time_us=")
            .and_then(|s| s.parse::<u64>().ok());

        assert_eq!(us_str, Some(1234567890));

        let ms = us_str.unwrap() / 1000;
        assert_eq!(ms, 1234567);
    }

    /// Test progress calculation
    #[test]
    fn test_progress_calculation() {
        let total_duration_ms = 30000u64; // 30 seconds
        let current_ms = 15000u64; // halfway
        let progress = (current_ms as f32 / total_duration_ms as f32).min(1.0);

        assert!((progress - 0.5).abs() < 0.01);
    }

    /// Test silence detection parsing
    #[test]
    fn test_parse_silence_output() {
        let stderr_sample = r#"[silencedetect @ 0x123] silence_start: 0.5
[silencedetect @ 0x123] silence_end: 2.5 | silence_duration: 2.0
[silencedetect @ 0x123] silence_start: 10.0
[silencedetect @ 0x123] silence_end: 12.0 | silence_duration: 2.0"#;

        let mut silence_spans = Vec::new();
        let mut current_start: Option<f64> = None;

        for line in stderr_sample.lines() {
            if line.contains("silence_start:") {
                if let Some(pos) = line.find("silence_start:") {
                    let after = line[pos + 14..].trim();
                    if let Ok(start) = after.parse::<f64>() {
                        current_start = Some(start);
                    }
                }
            } else if line.contains("silence_end:") {
                if let Some(pos) = line.find("silence_end:") {
                    let after = line[pos + 12..].trim();
                    if let Some(end) = after
                        .split('|')
                        .next()
                        .map(|s| s.trim())
                        .and_then(|s| s.parse::<f64>().ok())
                    {
                        if let Some(start) = current_start {
                            silence_spans.push(SilencePoint {
                                start_ms: (start * 1000.0) as u64,
                                end_ms: (end * 1000.0) as u64,
                            });
                        }
                    }
                }
            }
        }

        assert_eq!(silence_spans.len(), 2);
        assert_eq!(silence_spans[0].start_ms, 500);
        assert_eq!(silence_spans[0].end_ms, 2500);
        assert_eq!(silence_spans[1].start_ms, 10000);
        assert_eq!(silence_spans[1].end_ms, 12000);
    }

    /// Test ProbeResult construction
    #[test]
    fn test_probe_result() {
        let result = ProbeResult {
            duration_seconds: 120.5,
            has_audio: true,
            audio_codec: Some("opus".into()),
            file_size_bytes: 5_000_000,
            nb_streams: 2,
        };

        assert_eq!(result.duration_seconds, 120.5);
        assert!(result.has_audio);
        assert_eq!(result.audio_codec, Some("opus".into()));
        assert_eq!(result.file_size_bytes, 5_000_000);
    }

    /// Test ExtractStats construction
    #[test]
    fn test_extract_stats() {
        let stats = ExtractStats {
            output_size_bytes: 1_000_000,
            noise_reduction_applied: true,
        };

        assert_eq!(stats.output_size_bytes, 1_000_000);
        assert!(stats.noise_reduction_applied);
    }
}
