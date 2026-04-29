// ============================================================================
// Chunking Module
// ============================================================================
// Handles fallback chunking for audio files >100 MB (Groq limit)
// - Parse silence points from ffmpeg silencedetect
// - Calculate optimal chunk boundaries with overlap
// - Cut chunks using ffmpeg
// - Track chunk metadata for stitching

use crate::adapters::ffmpeg::SilencePoint;
use crate::types::AppErrorView;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Types
// ============================================================================

/// Metadata for a single audio chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioChunk {
    /// Absolute start time in milliseconds
    pub start_ms: u64,
    /// Absolute end time in milliseconds
    pub end_ms: u64,
    /// Overlap start (for deduplication in stitching)
    pub overlap_start_ms: Option<u64>,
    /// Overlap end (for deduplication in stitching)
    pub overlap_end_ms: Option<u64>,
    /// Path to chunk file
    pub path: PathBuf,
    /// Index in chunk list
    pub idx: u32,
    /// Total chunks
    pub total: u32,
}

impl AudioChunk {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms - self.start_ms
    }

    pub fn with_overlap(mut self, overlap_start: Option<u64>, overlap_end: Option<u64>) -> Self {
        self.overlap_start_ms = overlap_start;
        self.overlap_end_ms = overlap_end;
        self
    }
}

// ============================================================================
// Chunking Logic
// ============================================================================

const MAX_CHUNK_SIZE_BYTES: u64 = 80 * 1024 * 1024; // 80 MB
const GROQ_FILE_LIMIT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB limit

/// Determine if chunking is needed based on extracted audio size
pub fn should_chunk(audio_size_bytes: u64) -> bool {
    audio_size_bytes > GROQ_FILE_LIMIT_BYTES
}

/// Calculate optimal chunk boundaries using silence points
/// Returns list of (start_ms, end_ms) ranges
pub fn calculate_chunk_boundaries(
    total_duration_ms: u64,
    silence_points: &[SilencePoint],
    estimated_bitrate_kbps: f64,
) -> Result<Vec<(u64, u64)>, AppErrorView> {
    if silence_points.is_empty() {
        // No silence detected, use fixed-size chunks
        return calculate_fixed_size_boundaries(total_duration_ms, estimated_bitrate_kbps);
    }

    let mut boundaries = Vec::new();
    let mut current_start_ms = 0u64;

    // Try to fit silence boundaries into max chunk size
    let silence_boundaries: Vec<u64> = silence_points
        .iter()
        .flat_map(|sp| vec![sp.start_ms, sp.end_ms])
        .collect();

    // Start from beginning and accumulate until we exceed max chunk size
    let max_chunk_duration_ms =
        estimate_duration_from_size(MAX_CHUNK_SIZE_BYTES, estimated_bitrate_kbps);

    for &silence_boundary in &silence_boundaries {
        if silence_boundary > current_start_ms {
            let accumulated_ms = silence_boundary - current_start_ms;

            if accumulated_ms >= max_chunk_duration_ms && silence_boundary > current_start_ms {
                // Close chunk at this silence point
                boundaries.push((current_start_ms, silence_boundary));
                current_start_ms = silence_boundary;
            }
        }
    }

    // Add final chunk
    if current_start_ms < total_duration_ms {
        boundaries.push((current_start_ms, total_duration_ms));
    }

    // If no boundaries, fall back to fixed-size
    if boundaries.is_empty() {
        return calculate_fixed_size_boundaries(total_duration_ms, estimated_bitrate_kbps);
    }

    Ok(boundaries)
}

/// Calculate fixed-size chunk boundaries when silence detection fails
fn calculate_fixed_size_boundaries(
    total_duration_ms: u64,
    estimated_bitrate_kbps: f64,
) -> Result<Vec<(u64, u64)>, AppErrorView> {
    let max_chunk_duration_ms =
        estimate_duration_from_size(MAX_CHUNK_SIZE_BYTES, estimated_bitrate_kbps);

    let mut boundaries = Vec::new();
    let mut current_start_ms = 0u64;

    while current_start_ms < total_duration_ms {
        let chunk_end = (current_start_ms + max_chunk_duration_ms).min(total_duration_ms);
        boundaries.push((current_start_ms, chunk_end));
        current_start_ms = chunk_end;
    }

    Ok(boundaries)
}

/// Estimate duration (in ms) from file size and bitrate
/// Formula: duration_ms = (size_bytes * 8) / (bitrate_kbps * 1000)
fn estimate_duration_from_size(size_bytes: u64, bitrate_kbps: f64) -> u64 {
    if bitrate_kbps <= 0.0 {
        return 0;
    }
    let duration_secs = (size_bytes as f64 * 8.0) / (bitrate_kbps * 1000.0);
    (duration_secs * 1000.0) as u64
}

/// Estimate bitrate from audio metadata
/// For Opus 16kHz mono: typically 16-32 kbps
/// We'll use a default of 32 kbps if not specified
pub fn estimate_bitrate_kbps(audio_size_bytes: u64, duration_ms: u64) -> f64 {
    if duration_ms == 0 {
        return 32.0; // fallback
    }
    let duration_secs = duration_ms as f64 / 1000.0;
    (audio_size_bytes as f64 * 8.0) / (duration_secs * 1000.0)
}

/// Add overlap regions to chunk boundaries
pub fn add_overlaps(
    boundaries: Vec<(u64, u64)>,
    overlap_ms: u64,
) -> Vec<(u64, u64, Option<u64>, Option<u64>)> {
    let mut result = Vec::new();

    for (idx, (start, end)) in boundaries.iter().enumerate() {
        let mut overlap_start = None;
        let mut overlap_end = None;

        // Add overlap from previous chunk
        if idx > 0 && *start > overlap_ms {
            overlap_start = Some(start - overlap_ms);
        }

        // Add overlap to next chunk
        if idx < boundaries.len() - 1 {
            overlap_end = Some(end + overlap_ms);
        }

        result.push((*start, *end, overlap_start, overlap_end));
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_chunk() {
        assert!(!should_chunk(50 * 1024 * 1024)); // 50 MB < 100 MB
        assert!(!should_chunk(100 * 1024 * 1024)); // exactly 100 MB (not over limit)
        assert!(should_chunk(100 * 1024 * 1024 + 1)); // 100 MB + 1 byte
        assert!(should_chunk(150 * 1024 * 1024)); // 150 MB > 100 MB
    }

    #[test]
    fn test_estimate_bitrate() {
        // For a 1 hour (3600s) audio at 32kbps: ~14.4 MB
        let size_bytes = 14_400_000u64;
        let duration_ms = 3_600_000u64;
        let bitrate = estimate_bitrate_kbps(size_bytes, duration_ms);

        // Should be close to 32 kbps
        assert!((bitrate - 32.0).abs() < 2.0);
    }

    #[test]
    fn test_estimate_duration_from_size() {
        // For 32 kbps: 1 second = 4000 bytes
        // So 4MB = 1000 seconds = 1000000 ms
        let size_bytes = 4_000_000u64;
        let bitrate_kbps = 32.0;
        let duration_ms = estimate_duration_from_size(size_bytes, bitrate_kbps);

        assert_eq!(duration_ms, 1_000_000); // 1000 seconds = 1000000 ms
    }

    #[test]
    fn test_calculate_fixed_size_boundaries() {
        // Total duration: 2 hours (7200 seconds = 7200000 ms)
        // At 32 kbps: 2 hours = 28.8 MB
        // Max chunk: 80 MB at 32 kbps = ~20000 seconds
        // So we should get 1 chunk (total is less than max)
        let total_duration_ms = 7_200_000u64; // 2 hours
        let bitrate_kbps = 32.0;

        let boundaries = calculate_fixed_size_boundaries(total_duration_ms, bitrate_kbps)
            .expect("should calculate boundaries");

        // At 32 kbps, 80 MB = ~20000 seconds, which is > 2 hours
        // So we should have just one chunk
        assert_eq!(boundaries.len(), 1);
        assert_eq!(boundaries[0], (0, 7_200_000));
    }

    #[test]
    fn test_calculate_fixed_size_boundaries_large_file() {
        // Total duration: 24 hours (86400 seconds)
        // At 32 kbps: 24 hours = 345.6 MB
        // Max chunk: 80 MB at 32 kbps = ~20000 seconds
        // So we should get ~5 chunks
        let total_duration_ms = 86_400_000u64; // 24 hours
        let bitrate_kbps = 32.0;

        let boundaries = calculate_fixed_size_boundaries(total_duration_ms, bitrate_kbps)
            .expect("should calculate boundaries");

        // Should be multiple chunks
        assert!(boundaries.len() > 1);

        // All chunks should fit within total duration
        for (start, end) in &boundaries {
            assert!(*start < *end);
            assert!(*end <= total_duration_ms);
        }

        // Boundaries should be contiguous
        for i in 1..boundaries.len() {
            assert_eq!(boundaries[i].0, boundaries[i - 1].1);
        }
    }

    #[test]
    fn test_calculate_chunk_boundaries_with_silence() {
        // 30 minutes audio with some silence detected
        let total_duration_ms = 30 * 60 * 1000; // 1800000 ms
        let silence_points = vec![
            SilencePoint {
                start_ms: 300_000, // 5 minutes
                end_ms: 310_000,   // +10 seconds
            },
            SilencePoint {
                start_ms: 900_000, // 15 minutes
                end_ms: 910_000,
            },
        ];
        let bitrate_kbps = 32.0;

        let boundaries =
            calculate_chunk_boundaries(total_duration_ms, &silence_points, bitrate_kbps)
                .expect("should calculate boundaries");

        // Should be able to cut at silence points
        assert!(!boundaries.is_empty());

        // All boundaries should be within total duration
        for (start, end) in &boundaries {
            assert!(*start < *end);
            assert!(*end <= total_duration_ms);
        }
    }

    #[test]
    fn test_add_overlaps_single_chunk() {
        let boundaries = vec![(0, 10000)];
        let overlaps = add_overlaps(boundaries, 1000);

        // Single chunk should have no overlap regions
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].0, 0); // start
        assert_eq!(overlaps[0].1, 10000); // end
        assert_eq!(overlaps[0].2, None); // no overlap_start
        assert_eq!(overlaps[0].3, None); // no overlap_end
    }

    #[test]
    fn test_add_overlaps_multiple_chunks() {
        let boundaries = vec![(0, 5000), (5000, 10000), (10000, 15000)];
        let overlaps = add_overlaps(boundaries, 500);

        assert_eq!(overlaps.len(), 3);

        // First chunk: no overlap_start, has overlap_end
        assert_eq!(overlaps[0].0, 0);
        assert_eq!(overlaps[0].1, 5000);
        assert_eq!(overlaps[0].2, None);
        assert_eq!(overlaps[0].3, Some(5500)); // end + 500

        // Middle chunk: has both overlaps
        assert_eq!(overlaps[1].0, 5000);
        assert_eq!(overlaps[1].1, 10000);
        assert_eq!(overlaps[1].2, Some(4500)); // start - 500
        assert_eq!(overlaps[1].3, Some(10500)); // end + 500

        // Last chunk: has overlap_start, no overlap_end
        assert_eq!(overlaps[2].0, 10000);
        assert_eq!(overlaps[2].1, 15000);
        assert_eq!(overlaps[2].2, Some(9500)); // start - 500
        assert_eq!(overlaps[2].3, None);
    }

    #[test]
    fn test_add_overlaps_respects_chunk_boundaries() {
        let boundaries = vec![(0, 1000), (1000, 2000)];
        let overlaps = add_overlaps(boundaries, 500);

        // Second chunk overlap should start from first chunk boundary
        // but adjusted for the overlap
        assert_eq!(overlaps[1].2, Some(500)); // 1000 - 500
    }
}
