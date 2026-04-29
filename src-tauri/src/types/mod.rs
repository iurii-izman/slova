#![allow(dead_code)] // Many types are prepared for future implementation phases

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// ============================================================================
// Job Identity & State Machine
// ============================================================================

/// Уникальный идентификатор задачи
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub Uuid);

impl JobId {
    pub fn new() -> Self {
        JobId(Uuid::new_v4())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Cache & Deduplication
// ============================================================================

/// Full content hash using BLAKE3 (hex string, 64 chars)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub String);

impl ContentHash {
    pub fn new(hash_hex: String) -> Self {
        ContentHash(hash_hex)
    }
}

/// Fingerprint of settings (language, prompt, model, etc.)
/// Used to determine if cache is valid when settings change
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SettingsFingerprint(pub String);

impl SettingsFingerprint {
    pub fn new(fingerprint_hex: String) -> Self {
        SettingsFingerprint(fingerprint_hex)
    }
}

/// Cache key: combination of content hash + settings fingerprint
/// Two files with same content and same settings should have same cache key
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(pub String);

impl CacheKey {
    pub fn new(content_hash: &ContentHash, settings_fingerprint: &SettingsFingerprint) -> Self {
        CacheKey(format!("{}-{}", content_hash.0, settings_fingerprint.0))
    }
}

/// Weak key for batch deduplication: size + mtime + hash of first 1MB
/// Used to quickly identify obvious duplicates within current enqueue batch
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeakKey(pub String);

impl WeakKey {
    pub fn new(size: u64, mtime: u64, partial_hash: &str) -> Self {
        WeakKey(format!("{}-{}-{}", size, mtime, partial_hash))
    }
}

/// Состояние задачи в pipeline'е
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum JobState {
    /// Добавлена в очередь, ожидает обработки
    Queued,

    /// Проверка файла через ffprobe
    Probing,

    /// Конвертация видео в аудио opus 16kHz
    Extracting { progress: f32 },

    /// Нарезка на чанки (если >100MB)
    Chunking { progress: f32 },

    /// Загрузка на Groq API
    Uploading {
        progress: f32,
        chunk_idx: u32,
        chunk_total: u32,
    },

    /// Транскрибация через Whisper
    Transcribing { chunk_idx: u32, chunk_total: u32 },

    /// Склейка чанков в финальный текст
    Stitching,

    /// Постобработка через Llama (опционально)
    Postprocessing,

    /// Успешно завершено
    Done {
        output_path: PathBuf,
        duration_ms: u64,
    },

    /// Результат получен из кэша (не было вызова API)
    Cached {
        output_path: PathBuf,
        duration_ms: u64,
    },

    /// Пропущено (найден дубликат в текущей очереди)
    Skipped { duplicate_of: JobId },

    /// Ошибка с деталями
    Failed { error: AppErrorView, attempts: u32 },

    /// Отменено пользователем
    Cancelled,

    /// На паузе
    Paused,
}

/// Основная структура задачи
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub source_path: PathBuf,
    pub display_name: String,
    pub size_bytes: u64,
    pub created_at: String, // ISO 8601
    pub state: JobState,
    pub settings_snapshot: JobSettings,
    pub content_hash: Option<String>, // SHA256 hex
}

// ============================================================================
// Settings & Configuration
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub language: String, // e.g. "ru", "en"
    pub output_format: ExportFormat,
    pub parallelism: u32,         // max concurrent jobs
    pub enable_postprocess: bool, // llama cleanup
    pub groq_model: String,       // whisper-large-v3-turbo
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "ru".into(),
            output_format: ExportFormat::Txt,
            parallelism: 3,
            enable_postprocess: false,
            groq_model: "whisper-large-v3-turbo".into(),
        }
    }
}

/// Снимок настроек на момент добавления задачи
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobSettings {
    pub language: String,
    pub output_format: ExportFormat,
    #[serde(default)]
    pub enable_postprocess: bool,
}

impl Default for JobSettings {
    fn default() -> Self {
        JobSettings {
            language: "ru".into(),
            output_format: ExportFormat::Txt,
            enable_postprocess: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Srt,
    Json,
}

// ============================================================================
// Transcript & Export
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transcript {
    pub job_id: JobId,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Full transcript with segments and metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullTranscript {
    pub job_id: JobId,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
    pub duration_ms: u64,
}

/// Segments data from Groq response (temporary during processing)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroqSegment {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

/// Full Groq transcription response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroqTranscription {
    pub text: String,
    pub segments: Vec<GroqSegment>,
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppErrorView {
    pub code: String, // e.g. "INVALID_FILE", "RATE_LIMIT", "AUTH_FAILED"
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>, // extra context
}

impl AppErrorView {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        AppErrorView {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

impl std::fmt::Display for AppErrorView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

// Common error codes
impl AppErrorView {
    pub fn invalid_file(reason: impl Into<String>) -> Self {
        Self::new("INVALID_FILE", reason)
    }

    pub fn rate_limit(retry_after: Option<u32>) -> Self {
        let msg = match retry_after {
            Some(secs) => format!("Rate limited. Retry after {}s", secs),
            None => "Rate limited. Retry after a delay".into(),
        };
        Self::new("RATE_LIMIT", msg)
    }

    pub fn auth_failed() -> Self {
        Self::new("AUTH_FAILED", "API key is invalid or not set")
    }

    pub fn network_error(reason: impl Into<String>) -> Self {
        Self::new("NETWORK_ERROR", reason)
    }

    pub fn fs_error(reason: impl Into<String>) -> Self {
        Self::new("FS_ERROR", reason)
    }

    pub fn internal_error(reason: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", reason)
    }
}

// ============================================================================
// Filtering & Querying
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>, // filter by state kind

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

// ============================================================================
// Events (emitted from backend)
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueTick {
    pub updates: Vec<JobUpdate>,
    pub ts: u64, // milliseconds since epoch
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobUpdate {
    pub id: JobId,
    pub state: JobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_uploaded: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitEvent {
    pub retry_after_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppErrorEvent {
    pub error: AppErrorView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
}
