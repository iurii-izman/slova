// ============================================================================
// FFmpeg Adapter
// ============================================================================
// Wrapper around ffmpeg/ffprobe sidecars for:
// - Video validation (ffprobe)
// - Audio extraction: MP4 → Opus 16kHz 32kbps
// - Silence detection for chunking
// - Noise reduction via rnnoise filter

use crate::types::AppErrorView;
use std::path::PathBuf;

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

    /// Validate MP4 file and get metadata (duration, audio tracks, etc.)
    /// Uses ffprobe with JSON output
    pub async fn probe(&self, _path: &std::path::Path) -> Result<ProbeResult, AppErrorView> {
        // TODO: execute ffprobe -v quiet -print_format json -show_format -show_streams input.mp4
        Err(AppErrorView::internal_error("probe not implemented"))
    }

    /// Extract audio from MP4 to Opus 16kHz 32kbps with noise reduction
    /// Command: ffmpeg -i input.mp4 -vn -ac 1 -ar 16000
    ///          -af "arnndn=m=rnnoise-models/cb.rnnn" -c:a libopus -b:a 32k output.opus
    pub async fn extract_audio(
        &self,
        _input: &std::path::Path,
        _output: &std::path::Path,
    ) -> Result<(), AppErrorView> {
        // TODO: implement
        Err(AppErrorView::internal_error(
            "extract_audio not implemented",
        ))
    }

    /// Detect silence points for chunking (fallback for >100MB files)
    /// Uses ffmpeg silencedetect filter
    pub async fn silence_detect(
        &self,
        _audio_path: &std::path::Path,
    ) -> Result<Vec<SilencePoint>, AppErrorView> {
        // TODO: implement
        Err(AppErrorView::internal_error(
            "silence_detect not implemented",
        ))
    }

    /// Cut audio at specified time ranges
    /// Returns path to cut segment
    pub async fn cut(
        &self,
        _audio_path: &std::path::Path,
        _start_ms: u64,
        _duration_ms: u64,
        _output: &std::path::Path,
    ) -> Result<(), AppErrorView> {
        // TODO: implement
        Err(AppErrorView::internal_error("cut not implemented"))
    }
}

/// Result from ffprobe (JSON schema)
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub duration_seconds: f64,
    pub has_audio: bool,
    pub audio_codec: Option<String>,
}

/// Silence point detected by ffmpeg silencedetect
#[derive(Debug, Clone)]
pub struct SilencePoint {
    pub start_ms: u64,
    pub end_ms: u64,
}
