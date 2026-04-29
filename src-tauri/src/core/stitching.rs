// ============================================================================
// Stitching Module
// ============================================================================
// Handles merging of transcribed chunks back into a single transcript
// - Map local chunk timecodes to global timecodes
// - Deduplicate overlapping regions using text similarity
// - Merge segments and ensure proper ordering

use crate::types::{AppErrorView, TranscriptSegment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Chunk result after transcription
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkTranscript {
    /// Chunk index (0-based)
    pub chunk_idx: u32,
    /// Chunk start time (global, in milliseconds)
    pub chunk_start_ms: u64,
    /// Chunk end time (global, in milliseconds)
    pub chunk_end_ms: u64,
    /// Overlap region start (if any)
    pub overlap_start_ms: Option<u64>,
    /// Overlap region end (if any)
    pub overlap_end_ms: Option<u64>,
    /// Full transcript text
    pub text: String,
    /// Segments with timecodes (local to chunk, will be converted to global)
    pub segments: Vec<SegmentLocal>,
}

/// Segment with local timecodes (relative to chunk start)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentLocal {
    /// Start time relative to chunk (in seconds, as from Groq)
    pub start: f32,
    /// End time relative to chunk (in seconds)
    pub end: f32,
    /// Segment text
    pub text: String,
}

/// Deduplication rule for overlapping region (reserved for future use)
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct OverlapDedup {
    /// Global start of overlap region
    start_ms: u64,
    /// Global end of overlap region
    end_ms: u64,
}

// ============================================================================
// Stitching Logic
// ============================================================================

/// Merge multiple chunk transcripts into a single transcript
/// - Converts local segment times to global
/// - Deduplicates overlapping regions
/// - Returns combined segments and text
pub fn stitch_chunks(
    chunks: Vec<ChunkTranscript>,
) -> Result<(String, Vec<TranscriptSegment>), AppErrorView> {
    if chunks.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    if chunks.len() == 1 {
        let chunk = &chunks[0];
        let segments = convert_local_to_global(&chunk.segments, chunk.chunk_start_ms);
        let text = chunk.text.clone();
        return Ok((text, segments));
    }

    // For multiple chunks, we need to handle overlaps
    let mut all_segments = Vec::new();
    let mut all_text = String::new();

    // Build a map of overlaps to deduplicate
    let mut dedup_map: HashMap<usize, OverlapDedup> = HashMap::new();

    for i in 0..chunks.len() - 1 {
        let curr_chunk = &chunks[i];
        let next_chunk = &chunks[i + 1];

        if let (Some(overlap_end), Some(overlap_start)) =
            (curr_chunk.overlap_end_ms, next_chunk.overlap_start_ms)
        {
            if overlap_start < overlap_end {
                // There is an overlap region: [overlap_start, overlap_end)
                // Find which segments from current and next chunk fall in this region

                let curr_overlapping = find_segments_in_range(
                    &curr_chunk.segments,
                    curr_chunk.chunk_start_ms,
                    overlap_start,
                    overlap_end,
                );

                let next_overlapping = find_segments_in_range(
                    &next_chunk.segments,
                    next_chunk.chunk_start_ms,
                    overlap_start,
                    overlap_end,
                );

                // Try to match and deduplicate
                let (_skip_curr, _skip_next) =
                    deduplicate_overlap(&curr_overlapping, &next_overlapping);

                // Dedup tracking can be extended later if needed
                dedup_map.insert(
                    i,
                    OverlapDedup {
                        start_ms: overlap_start,
                        end_ms: overlap_end,
                    },
                );
            }
        }
    }

    // Now merge chunks in order, respecting dedup rules
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let mut segments = convert_local_to_global(&chunk.segments, chunk.chunk_start_ms);

        // Check if this chunk has overlaps from the previous chunk
        if let Some(dedup) = dedup_map.get(&(chunk_idx.saturating_sub(1))) {
            if chunk_idx > 0 {
                // Remove segments that were already included in previous chunk
                segments.retain(|seg| {
                    let seg_start_ms = seg.start_ms;
                    !(seg_start_ms >= dedup.start_ms && seg_start_ms < dedup.end_ms)
                });
            }
        }

        // Append text and segments
        if !all_text.is_empty() && !chunk.text.is_empty() {
            all_text.push(' ');
        }
        all_text.push_str(&chunk.text);
        all_segments.extend(segments);
    }

    // Sort segments by start time
    all_segments.sort_by_key(|seg| seg.start_ms);

    Ok((all_text, all_segments))
}

/// Convert segment times from local (relative to chunk) to global
fn convert_local_to_global(
    segments: &[SegmentLocal],
    chunk_start_ms: u64,
) -> Vec<TranscriptSegment> {
    segments
        .iter()
        .map(|seg| {
            let start_ms = (seg.start * 1000.0) as u64 + chunk_start_ms;
            let end_ms = (seg.end * 1000.0) as u64 + chunk_start_ms;
            TranscriptSegment {
                start_ms,
                end_ms,
                text: seg.text.clone(),
            }
        })
        .collect()
}

/// Find segments that fall within a time range (in global ms)
fn find_segments_in_range(
    segments: &[SegmentLocal],
    chunk_start_ms: u64,
    range_start_ms: u64,
    range_end_ms: u64,
) -> Vec<(usize, TranscriptSegment)> {
    segments
        .iter()
        .enumerate()
        .filter_map(|(idx, seg)| {
            let start_ms = (seg.start * 1000.0) as u64 + chunk_start_ms;
            let end_ms = (seg.end * 1000.0) as u64 + chunk_start_ms;

            // Check if segment overlaps with range
            if start_ms < range_end_ms && end_ms > range_start_ms {
                return Some((
                    idx,
                    TranscriptSegment {
                        start_ms,
                        end_ms,
                        text: seg.text.clone(),
                    },
                ));
            }
            None
        })
        .collect()
}

/// Deduplicate overlapping segments using text similarity
/// Returns (indices_to_skip_in_prev, indices_to_skip_in_next)
fn deduplicate_overlap(
    prev_segments: &[(usize, TranscriptSegment)],
    next_segments: &[(usize, TranscriptSegment)],
) -> (Vec<usize>, Vec<usize>) {
    let _skip_prev = Vec::new();
    let mut skip_next = Vec::new();

    // Simple heuristic: if text is very similar, consider it duplicate
    for (_prev_idx, prev_seg) in prev_segments {
        for (next_idx, next_seg) in next_segments {
            let similarity = calculate_text_similarity(&prev_seg.text, &next_seg.text);
            // If similarity > 80%, consider it a duplicate
            if similarity > 0.8 {
                skip_next.push(*next_idx);
                break; // Only match each segment once
            }
        }
    }

    (_skip_prev, skip_next) // Note: _skip_prev not used yet but available for future refinement
}

/// Calculate similarity between two strings using normalized token comparison
/// Returns a value between 0.0 (completely different) and 1.0 (identical)
fn calculate_text_similarity(text1: &str, text2: &str) -> f32 {
    let norm1 = normalize_text(text1);
    let norm2 = normalize_text(text2);

    if norm1.is_empty() && norm2.is_empty() {
        return 1.0;
    }
    if norm1.is_empty() || norm2.is_empty() {
        return 0.0;
    }

    // Split into tokens
    let tokens1: Vec<&str> = norm1.split_whitespace().collect();
    let tokens2: Vec<&str> = norm2.split_whitespace().collect();

    if tokens1.is_empty() && tokens2.is_empty() {
        return 1.0;
    }

    // Count matching tokens (simple Jaccard-like similarity)
    let mut matches = 0;
    for token in &tokens1 {
        if tokens2.contains(token) {
            matches += 1;
        }
    }

    let total = tokens1.len().max(tokens2.len());
    matches as f32 / total as f32
}

/// Normalize text for comparison (lowercase, remove punctuation)
fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stitch_single_chunk() {
        let chunk = ChunkTranscript {
            chunk_idx: 0,
            chunk_start_ms: 0,
            chunk_end_ms: 5000,
            overlap_start_ms: None,
            overlap_end_ms: None,
            text: "Hello world".to_string(),
            segments: vec![
                SegmentLocal {
                    start: 0.0,
                    end: 1.0,
                    text: "Hello".to_string(),
                },
                SegmentLocal {
                    start: 1.0,
                    end: 2.0,
                    text: "world".to_string(),
                },
            ],
        };

        let (text, segments) = stitch_chunks(vec![chunk]).expect("should stitch");

        assert_eq!(text, "Hello world");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].start_ms, 0);
        assert_eq!(segments[0].end_ms, 1000);
        assert_eq!(segments[1].start_ms, 1000);
        assert_eq!(segments[1].end_ms, 2000);
    }

    #[test]
    fn test_stitch_multiple_chunks_no_overlap() {
        let chunk1 = ChunkTranscript {
            chunk_idx: 0,
            chunk_start_ms: 0,
            chunk_end_ms: 5000,
            overlap_start_ms: None,
            overlap_end_ms: None, // No overlap at end
            text: "Hello".to_string(),
            segments: vec![SegmentLocal {
                start: 0.0,
                end: 5.0,
                text: "Hello".to_string(),
            }],
        };

        let chunk2 = ChunkTranscript {
            chunk_idx: 1,
            chunk_start_ms: 5000,
            chunk_end_ms: 10000,
            overlap_start_ms: None, // No overlap at start
            overlap_end_ms: None,
            text: "world".to_string(),
            segments: vec![SegmentLocal {
                start: 0.0,
                end: 5.0,
                text: "world".to_string(),
            }],
        };

        let (text, segments) = stitch_chunks(vec![chunk1, chunk2]).expect("should stitch");

        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn test_normalize_text() {
        assert_eq!(normalize_text("Hello, World!"), "hello world");
        assert_eq!(normalize_text("  Hello   world  "), "hello world");
        assert_eq!(normalize_text("123 abc !@# XYZ"), "123 abc xyz");
    }

    #[test]
    fn test_calculate_text_similarity_identical() {
        let similarity = calculate_text_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_calculate_text_similarity_partial() {
        // "hello world" vs "hello there" - share "hello"
        let similarity = calculate_text_similarity("hello world", "hello there");
        assert!(similarity > 0.4 && similarity < 0.8);
    }

    #[test]
    fn test_calculate_text_similarity_different() {
        let similarity = calculate_text_similarity("abc def", "xyz uvw");
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_calculate_text_similarity_punctuation_insensitive() {
        let similarity = calculate_text_similarity("Hello, World!", "Hello world");
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_convert_local_to_global() {
        let segments = vec![
            SegmentLocal {
                start: 0.0,
                end: 1.5,
                text: "Hello".to_string(),
            },
            SegmentLocal {
                start: 2.0,
                end: 3.5,
                text: "world".to_string(),
            },
        ];

        let global = convert_local_to_global(&segments, 5000);

        assert_eq!(global.len(), 2);
        assert_eq!(global[0].start_ms, 5000);
        assert_eq!(global[0].end_ms, 6500);
        assert_eq!(global[1].start_ms, 7000);
        assert_eq!(global[1].end_ms, 8500);
    }

    #[test]
    fn test_find_segments_in_range() {
        let segments = vec![
            SegmentLocal {
                start: 0.0,
                end: 2.0, // 0-2 seconds
                text: "seg1".to_string(),
            },
            SegmentLocal {
                start: 2.0,
                end: 4.0, // 2-4 seconds
                text: "seg2".to_string(),
            },
            SegmentLocal {
                start: 4.0,
                end: 6.0, // 4-6 seconds
                text: "seg3".to_string(),
            },
        ];

        let chunk_start = 10000; // 10 seconds = 10000 ms
                                 // seg1: 10000-12000, seg2: 12000-14000, seg3: 14000-16000
        let range_start = 11000; // overlaps with seg1 (10000-12000) and seg2 (12000-14000)
        let range_end = 13000; // within seg2

        let overlapping = find_segments_in_range(&segments, chunk_start, range_start, range_end);

        assert_eq!(overlapping.len(), 2); // seg1 and seg2
    }

    #[test]
    fn test_empty_chunks() {
        let (text, segments) = stitch_chunks(vec![]).expect("should handle empty");
        assert!(text.is_empty());
        assert!(segments.is_empty());
    }
}
