// ============================================================================
// Export Layer
// ============================================================================
// Handles transcript formatting and atomic file writing:
// - TXT: plain text format (with optional edited version)
// - SRT: SubRip subtitle format with proper timecode formatting
// - JSON: complete segments and metadata
// - Atomic write: write to temp, then rename to prevent corruption
// - Conflict policy: overwrite, suffix (unique), or skip

use crate::types::{AppErrorView, TranscriptSegment};
use std::path::{Path, PathBuf};
use tokio::fs;

// ============================================================================
// Conflict Policy for File Writes
// ============================================================================

#[derive(Clone, Debug, Default)]
pub enum ConflictPolicy {
    /// Overwrite existing file
    #[default]
    Overwrite,
    /// Add suffix if file exists (e.g., video.srt → video.1.srt)
    Suffix,
    /// Skip if file exists
    Skip,
}

// ============================================================================
// SRT Formatter
// ============================================================================

/// Format timecode as HH:MM:SS,mmm (SRT standard)
pub fn format_timecode(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

/// Convert segments to SRT format with proper subtitle indexing and line breaks
pub fn format_srt(segments: &[TranscriptSegment]) -> String {
    let mut result = String::new();
    let mut current_index = 1;
    let mut current_start = 0u64;
    let mut current_text = String::new();

    for segment in segments {
        // If there's a gap or text is long, start a new subtitle
        let gap_ms = segment.start_ms.saturating_sub(current_start + 5000); // 5 sec threshold
        let should_split = gap_ms > 0 || current_text.len() + segment.text.len() > 100;

        if should_split && !current_text.is_empty() {
            // Write current subtitle
            result.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                current_index,
                format_timecode(current_start),
                format_timecode(
                    segments
                        .iter()
                        .take_while(|s| s.start_ms < segment.start_ms)
                        .last()
                        .map(|s| s.end_ms)
                        .unwrap_or(current_start)
                ),
                current_text.trim()
            ));
            current_index += 1;
            current_text.clear();
        }

        // Add text with proper word wrapping
        if !current_text.is_empty() {
            current_text.push(' ');
        }
        current_text.push_str(&segment.text);

        if current_text.is_empty() {
            current_start = segment.start_ms;
        }
    }

    // Write last subtitle
    if !current_text.is_empty() {
        let last_end = segments.last().map(|s| s.end_ms).unwrap_or(current_start);
        result.push_str(&format!(
            "{}\n{} --> {}\n{}\n",
            current_index,
            format_timecode(current_start),
            format_timecode(last_end),
            current_text.trim()
        ));
    }

    result
}

/// Better SRT formatting with intelligent line breaking and segment grouping
pub fn format_srt_advanced(segments: &[TranscriptSegment]) -> String {
    if segments.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut subs = Vec::new();

    let mut current_start = segments[0].start_ms;
    let mut current_end = segments[0].end_ms;
    let mut current_lines: Vec<String> = vec![segments[0].text.clone()];

    for (_i, segment) in segments.iter().enumerate().skip(1) {
        let gap = segment.start_ms.saturating_sub(current_end);

        // Check if we should merge with current or start new subtitle
        // Merge if: gap < 1 sec AND total text < 100 chars
        let current_text = current_lines.join(" ");
        let merged_text = format!("{} {}", current_text, segment.text);
        let should_merge = gap < 1000 && merged_text.len() < 100;

        if should_merge {
            current_lines.push(segment.text.clone());
            current_end = segment.end_ms;
        } else {
            // Finalize current subtitle
            subs.push((current_start, current_end, current_lines.join(" ")));

            // Start new subtitle
            current_start = segment.start_ms;
            current_end = segment.end_ms;
            current_lines = vec![segment.text.clone()];
        }
    }

    // Finalize last subtitle
    subs.push((current_start, current_end, current_lines.join(" ")));

    // Format as SRT
    for (idx, (start, end, text)) in subs.into_iter().enumerate() {
        result.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            idx + 1,
            format_timecode(start),
            format_timecode(end),
            text.trim()
        ));
    }

    result
}

// ============================================================================
// Format Functions
// ============================================================================

/// Format transcript as plain text
pub fn format_txt(text: &str) -> String {
    text.to_string()
}

/// Format transcript as JSON with segments and metadata
pub fn format_json(text: &str, segments: &[TranscriptSegment]) -> String {
    let json = serde_json::json!({
        "text": text,
        "segments": segments.iter().map(|s| {
            serde_json::json!({
                "start_ms": s.start_ms,
                "end_ms": s.end_ms,
                "text": s.text,
            })
        }).collect::<Vec<_>>(),
        "duration_ms": segments.last().map(|s| s.end_ms).unwrap_or(0),
    });

    serde_json::to_string_pretty(&json).unwrap_or_else(|_| text.to_string())
}

// ============================================================================
// Atomic Write with Conflict Resolution
// ============================================================================

/// Generate unique path with suffix if file exists
async fn unique_with_suffix(target: &Path) -> Result<PathBuf, AppErrorView> {
    let mut counter = 1;
    loop {
        let stem = target
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| AppErrorView::fs_error("Invalid file path"))?;

        let ext = target.extension().and_then(|s| s.to_str()).unwrap_or("");

        let new_name = if ext.is_empty() {
            format!("{}.{}", stem, counter)
        } else {
            format!("{}.{}.{}", stem, counter, ext)
        };

        let new_path = target.parent().map(|p| p.join(&new_name));

        if let Some(path) = new_path {
            if !path.exists() {
                return Ok(path);
            }
        }

        counter += 1;
        if counter > 1000 {
            return Err(AppErrorView::fs_error(
                "Too many existing files, cannot generate unique name",
            ));
        }
    }
}

/// Write data to file atomically with conflict resolution
pub async fn write_atomic(
    target: &Path,
    data: &[u8],
    conflict: ConflictPolicy,
) -> Result<PathBuf, AppErrorView> {
    // Determine final path based on conflict policy
    let final_path = match conflict {
        ConflictPolicy::Overwrite => target.to_path_buf(),
        ConflictPolicy::Suffix => {
            if target.exists() {
                unique_with_suffix(target).await?
            } else {
                target.to_path_buf()
            }
        }
        ConflictPolicy::Skip => {
            if target.exists() {
                return Err(AppErrorView::new(
                    "FILE_EXISTS",
                    format!("File already exists: {}", target.display()),
                ));
            }
            target.to_path_buf()
        }
    };

    // Ensure parent directory exists
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| AppErrorView::fs_error(format!("Failed to create directory: {}", e)))?;
    }

    // Write to temporary file
    let tmp_path = final_path.with_extension("tmp");
    fs::write(&tmp_path, data)
        .await
        .map_err(|e| AppErrorView::fs_error(format!("Failed to write temporary file: {}", e)))?;

    // Atomic rename
    fs::rename(&tmp_path, &final_path).await.map_err(|e| {
        // Clean up temp file on error
        let _ = std::fs::remove_file(&tmp_path);
        AppErrorView::fs_error(format!("Failed to finalize write: {}", e))
    })?;

    Ok(final_path)
}

// ============================================================================
// High-Level Export Functions
// ============================================================================

/// Export transcript in specified format to file
pub async fn export_transcript(
    text: &str,
    segments: &[TranscriptSegment],
    format: &str,
    target_path: &Path,
    conflict: ConflictPolicy,
) -> Result<PathBuf, AppErrorView> {
    let content = match format {
        "txt" => format_txt(text),
        "srt" => format_srt_advanced(segments),
        "json" => format_json(text, segments),
        _ => {
            return Err(AppErrorView::new(
                "INVALID_FORMAT",
                format!("Unknown format: {}", format),
            ))
        }
    };

    // Set correct extension
    let path_with_ext = target_path.with_extension(format);
    write_atomic(&path_with_ext, content.as_bytes(), conflict).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timecode() {
        assert_eq!(format_timecode(0), "00:00:00,000");
        assert_eq!(format_timecode(1000), "00:00:01,000");
        assert_eq!(format_timecode(61000), "00:01:01,000");
        assert_eq!(format_timecode(3661000), "01:01:01,000");
        assert_eq!(format_timecode(3661500), "01:01:01,500");
    }

    #[test]
    fn test_format_srt_advanced() {
        let segments = vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 1000,
                text: "Hello".to_string(),
            },
            TranscriptSegment {
                start_ms: 1500,
                end_ms: 2500,
                text: "world".to_string(),
            },
            TranscriptSegment {
                start_ms: 2600,
                end_ms: 5000,
                text: "This is a test sentence that should be merged.".to_string(),
            },
        ];

        let srt = format_srt_advanced(&segments);

        // Should contain proper SRT format
        assert!(srt.contains("00:00:00,000"));
        assert!(srt.contains("-->")); // Arrow separator
        assert!(srt.contains("1\n")); // Subtitle index

        // Check content preservation
        assert!(srt.contains("Hello"));
        assert!(srt.contains("world"));
    }

    #[test]
    fn test_format_json() {
        let segments = vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 1000,
            text: "Test".to_string(),
        }];

        let json_str = format_json("Test", &segments);

        // Verify it's valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&json_str).is_ok());
        assert!(json_str.contains("\"text\": \"Test\""));
    }

    #[test]
    fn test_format_txt() {
        let text = "This is a test transcript.";
        let result = format_txt(text);
        assert_eq!(result, text);
    }

    #[tokio::test]
    async fn test_write_atomic_creates_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        let result = write_atomic(&target, b"test content", ConflictPolicy::Overwrite).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_write_atomic_overwrite() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        // Write first
        let _ = write_atomic(&target, b"first", ConflictPolicy::Overwrite).await;

        // Overwrite
        let result = write_atomic(&target, b"second", ConflictPolicy::Overwrite).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "second");
    }

    #[tokio::test]
    async fn test_write_atomic_suffix() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        // Write first
        let _ = write_atomic(&target, b"first", ConflictPolicy::Overwrite).await;

        // Write with suffix policy
        let result = write_atomic(&target, b"second", ConflictPolicy::Suffix).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("test."));
        assert_ne!(path, target); // Should be different path

        // Original file should be unchanged
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "first");
    }

    #[tokio::test]
    async fn test_write_atomic_skip() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        // Write first
        let _ = write_atomic(&target, b"first", ConflictPolicy::Overwrite).await;

        // Try to write with skip policy
        let result = write_atomic(&target, b"second", ConflictPolicy::Skip).await;
        assert!(result.is_err());

        // Original file should be unchanged
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "first");
    }

    #[tokio::test]
    async fn test_export_transcript_all_formats() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let segments = vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 1000,
            text: "Hello world".to_string(),
        }];

        let text = "Hello world";

        // Test TXT
        let result = export_transcript(
            text,
            &segments,
            "txt",
            &temp_dir.path().join("test"),
            ConflictPolicy::Overwrite,
        )
        .await;
        assert!(result.is_ok());
        assert!(result.unwrap().with_extension("txt").exists());

        // Test SRT
        let result = export_transcript(
            text,
            &segments,
            "srt",
            &temp_dir.path().join("test"),
            ConflictPolicy::Overwrite,
        )
        .await;
        assert!(result.is_ok());
        assert!(result.unwrap().with_extension("srt").exists());

        // Test JSON
        let result = export_transcript(
            text,
            &segments,
            "json",
            &temp_dir.path().join("test"),
            ConflictPolicy::Overwrite,
        )
        .await;
        assert!(result.is_ok());
        assert!(result.unwrap().with_extension("json").exists());
    }
}
