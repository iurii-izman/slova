// ============================================================================
// Caching & Deduplication System
// ============================================================================
// Provides:
// - BLAKE3 full content hash for reliable deduplication
// - Settings fingerprint to detect cache invalidation
// - Cache key generation (hash + settings)
// - Weak key for batch-level deduplication (size + mtime + partial hash)
// - Non-blocking async hashing for large files
// - Cache validation before Groq API calls

use crate::types::AppErrorView;
use crate::types::{CacheKey, ContentHash, JobSettings, SettingsFingerprint, WeakKey};
use std::path::Path;

/// Maximum size for reading at once (1MB for weak key hashing)
const WEAK_KEY_READ_SIZE: usize = 1024 * 1024;

/// ============================================================================
/// Content Hash (BLAKE3 full file)
/// ============================================================================
/// Calculate BLAKE3 hash of entire file
/// Spawned in separate tokio task to avoid blocking UI
pub async fn calculate_content_hash(file_path: &Path) -> Result<ContentHash, AppErrorView> {
    let path = file_path.to_path_buf();

    let hash = tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path).map_err(|e| {
            AppErrorView::fs_error(format!("Failed to open file for hashing: {}", e))
        })?;

        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0u8; 65536]; // 64KB buffer

        loop {
            let bytes_read = std::io::Read::read(&mut file, &mut buffer).map_err(|e| {
                AppErrorView::fs_error(format!("Failed to read file for hashing: {}", e))
            })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        Ok::<_, AppErrorView>(hasher.finalize().to_hex().to_string())
    })
    .await
    .map_err(|e| AppErrorView::internal_error(format!("Hashing task panicked: {}", e)))??;

    Ok(ContentHash::new(hash))
}

/// ============================================================================
/// Settings Fingerprint
/// ============================================================================
/// Create fingerprint of settings (excludes non-functional settings)
/// Changes to language, prompt, or model invalidate cache
pub fn settings_fingerprint(settings: &JobSettings) -> Result<SettingsFingerprint, AppErrorView> {
    let fingerprint_data = format!(
        "{}|{}",
        settings.language,
        settings.output_format as u8 // Simple enum representation
    );

    let hash = blake3::hash(fingerprint_data.as_bytes())
        .to_hex()
        .to_string();
    Ok(SettingsFingerprint::new(hash))
}

/// ============================================================================
/// Cache Key Generation
/// ============================================================================
/// Generate cache key: content_hash + settings_fingerprint
pub async fn generate_cache_key(
    file_path: &Path,
    settings: &JobSettings,
) -> Result<CacheKey, AppErrorView> {
    let content_hash = calculate_content_hash(file_path).await?;
    let settings_fp = settings_fingerprint(settings)?;

    Ok(CacheKey::new(&content_hash, &settings_fp))
}

/// ============================================================================
/// Weak Key (Batch Deduplication)
/// ============================================================================
/// Quick hash of first 1MB for batch-level deduplication
async fn calculate_weak_hash(file_path: &Path) -> Result<String, AppErrorView> {
    let path = file_path.to_path_buf();

    let hash = tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path)
            .map_err(|e| AppErrorView::fs_error(format!("Failed to open file: {}", e)))?;

        let mut buffer = vec![0u8; WEAK_KEY_READ_SIZE];
        let bytes_read = std::io::Read::read(&mut file, &mut buffer)
            .map_err(|e| AppErrorView::fs_error(format!("Failed to read file: {}", e)))?;

        buffer.truncate(bytes_read);
        Ok::<_, AppErrorView>(blake3::hash(&buffer).to_hex().to_string())
    })
    .await
    .map_err(|e| AppErrorView::internal_error(format!("Task panicked: {}", e)))??;

    Ok(hash)
}

/// Generate weak key for deduplication: size + mtime + partial hash
pub async fn generate_weak_key(file_path: &Path) -> Result<WeakKey, AppErrorView> {
    let metadata = tokio::fs::metadata(file_path)
        .await
        .map_err(|e| AppErrorView::fs_error(format!("Failed to stat file: {}", e)))?;

    let size = metadata.len();
    let mtime = metadata
        .modified()
        .map_err(|e| AppErrorView::fs_error(format!("Failed to get mtime: {}", e)))?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppErrorView::fs_error(format!("Time error: {}", e)))?
        .as_secs();

    let partial_hash = calculate_weak_hash(file_path).await?;

    Ok(WeakKey::new(size, mtime, &partial_hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExportFormat;

    #[tokio::test]
    async fn test_content_hash_generation() {
        // Create temporary test file
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.bin");
        std::fs::write(&test_file, b"Hello, world!").unwrap();

        let hash = calculate_content_hash(&test_file).await.unwrap();

        // Hash should be consistent (same content = same hash)
        let hash2 = calculate_content_hash(&test_file).await.unwrap();
        assert_eq!(hash.0, hash2.0);

        // Hash should be 64 chars (BLAKE3 hex output)
        assert_eq!(hash.0.len(), 64);
    }

    #[tokio::test]
    async fn test_content_hash_different_files() {
        let temp_dir = tempfile::tempdir().unwrap();

        let file1 = temp_dir.path().join("file1.bin");
        let file2 = temp_dir.path().join("file2.bin");

        std::fs::write(&file1, b"Content 1").unwrap();
        std::fs::write(&file2, b"Content 2").unwrap();

        let hash1 = calculate_content_hash(&file1).await.unwrap();
        let hash2 = calculate_content_hash(&file2).await.unwrap();

        assert_ne!(hash1.0, hash2.0);
    }

    #[test]
    fn test_settings_fingerprint() {
        let settings1 = JobSettings {
            language: "ru".to_string(),
            output_format: ExportFormat::Txt,
            enable_postprocess: false,
        };

        let settings2 = JobSettings {
            language: "en".to_string(),
            output_format: ExportFormat::Txt,
            enable_postprocess: false,
        };

        let fp1 = settings_fingerprint(&settings1).unwrap();
        let fp2 = settings_fingerprint(&settings2).unwrap();

        // Different languages should produce different fingerprints
        assert_ne!(fp1.0, fp2.0);
    }

    #[test]
    fn test_settings_fingerprint_consistency() {
        let settings = JobSettings {
            language: "ru".to_string(),
            output_format: ExportFormat::Txt,
            enable_postprocess: false,
        };

        let fp1 = settings_fingerprint(&settings).unwrap();
        let fp2 = settings_fingerprint(&settings).unwrap();

        assert_eq!(fp1.0, fp2.0);
    }

    #[test]
    fn test_cache_key() {
        let hash1 = ContentHash::new("abc123".to_string());
        let hash2 = ContentHash::new("def456".to_string());

        let fp1 = SettingsFingerprint::new("fp1".to_string());
        let _fp2 = SettingsFingerprint::new("fp2".to_string());

        let key1 = CacheKey::new(&hash1, &fp1);
        let key2 = CacheKey::new(&hash1, &fp1);
        let key3 = CacheKey::new(&hash2, &fp1);

        // Same hash + same fp = same key
        assert_eq!(key1.0, key2.0);
        // Different hash = different key
        assert_ne!(key1.0, key3.0);
    }

    #[tokio::test]
    async fn test_weak_key_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.bin");
        std::fs::write(&test_file, b"Test content").unwrap();

        let weak_key = generate_weak_key(&test_file).await.unwrap();

        // Weak key should contain size + mtime + hash
        assert!(weak_key.0.contains('-'));
    }

    #[tokio::test]
    async fn test_weak_key_different_files() {
        let temp_dir = tempfile::tempdir().unwrap();

        let file1 = temp_dir.path().join("file1.bin");
        let file2 = temp_dir.path().join("file2.bin");

        std::fs::write(&file1, b"Content 1").unwrap();
        std::fs::write(&file2, b"Content 2").unwrap();

        let key1 = generate_weak_key(&file1).await.unwrap();
        let key2 = generate_weak_key(&file2).await.unwrap();

        assert_ne!(key1.0, key2.0);
    }
}
