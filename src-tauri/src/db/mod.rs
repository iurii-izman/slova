// ============================================================================
// Database Layer
// ============================================================================
// SQLite persistence for:
// - Job history (created, updated, state transitions)
// - Transcript cache (edits, versions)
// - Settings snapshots (per job)
// - File content hashes (for deduplication)
// - Failed job logs (for retry analysis)

mod migrations;

#[cfg(test)]
mod tests;

use crate::types::{AppErrorView, Job, JobFilter, JobId, JobSettings, JobState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;

/// Database pool handle
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    /// Initialize SQLite database and run migrations
    pub async fn init(db_path: &Path) -> Result<Self, AppErrorView> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppErrorView::fs_error(format!("Failed to create db directory: {}", e))
            })?;
        }

        let db_url = format!(
            "sqlite://{}",
            db_path
                .to_str()
                .ok_or_else(|| AppErrorView::internal_error("Invalid database path"))?
        );

        // Configure SQLite connection with sensible defaults
        let connect_opts = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| AppErrorView::internal_error(format!("Invalid SQLite URL: {}", e)))?
            .create_if_missing(true);

        // Create pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_opts)
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to connect to database: {}", e))
            })?;

        // Run migrations
        migrations::run(&pool)
            .await
            .map_err(|e| AppErrorView::internal_error(format!("Migration failed: {}", e)))?;

        Ok(Database { pool })
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), AppErrorView> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Database health check failed: {}", e))
            })?;
        Ok(())
    }
}

// ============================================================================
// Repositories
// ============================================================================

/// Job history and state repository
pub struct JobRepo {
    pool: SqlitePool,
}

impl JobRepo {
    pub fn new(pool: SqlitePool) -> Self {
        JobRepo { pool }
    }

    /// Insert a new job record
    pub async fn insert(&self, job: &Job) -> Result<(), AppErrorView> {
        let state_json = serde_json::to_string(&job.state).map_err(|e| {
            AppErrorView::internal_error(format!("Failed to serialize state: {}", e))
        })?;

        let settings_json = serde_json::to_string(&job.settings_snapshot).map_err(|e| {
            AppErrorView::internal_error(format!("Failed to serialize settings: {}", e))
        })?;

        sqlx::query(
            "INSERT INTO jobs (id, source_path, display_name, size_bytes, content_hash, created_at, state, state_payload, settings_json, attempts)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(job.id.0.to_string())
        .bind(job.source_path.to_string_lossy().to_string())
        .bind(&job.display_name)
        .bind(job.size_bytes as i64)
        .bind(job.content_hash.as_deref())
        .bind(&job.created_at)
        .bind("Queued")
        .bind(&state_json)
        .bind(&settings_json)
        .bind(0i32)
        .execute(&self.pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to insert job: {}", e)))?;

        Ok(())
    }

    /// Update job state
    pub async fn update_state(&self, id: JobId, state: &JobState) -> Result<(), AppErrorView> {
        let state_json = serde_json::to_string(state).map_err(|e| {
            AppErrorView::internal_error(format!("Failed to serialize state: {}", e))
        })?;

        let state_kind = match state {
            JobState::Queued => "Queued",
            JobState::Probing => "Probing",
            JobState::Extracting { .. } => "Extracting",
            JobState::Chunking { .. } => "Chunking",
            JobState::Uploading { .. } => "Uploading",
            JobState::Transcribing { .. } => "Transcribing",
            JobState::Stitching => "Stitching",
            JobState::Postprocessing => "Postprocessing",
            JobState::Done { .. } => "Done",
            JobState::Failed { .. } => "Failed",
            JobState::Cancelled => "Cancelled",
            JobState::Paused => "Paused",
        };

        sqlx::query("UPDATE jobs SET state = ?, state_payload = ? WHERE id = ?")
            .bind(state_kind)
            .bind(&state_json)
            .bind(id.0.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to update job state: {}", e))
            })?;

        Ok(())
    }

    /// Query jobs with filter
    pub async fn list(&self, filter: Option<JobFilter>) -> Result<Vec<Job>, AppErrorView> {
        let mut query = "SELECT id, source_path, display_name, size_bytes, content_hash, created_at, state, state_payload, settings_json
                        FROM jobs WHERE 1=1"
            .to_string();

        if let Some(ref f) = filter {
            if let Some(state) = &f.state {
                query.push_str(&format!(" AND state = '{}'", state));
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(f) = &filter {
            if let Some(limit) = f.limit {
                query.push_str(&format!(" LIMIT {}", limit));
            }
            if let Some(offset) = f.offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }
        }

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppErrorView::internal_error(format!("Failed to list jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let state_json: String = row.get("state_payload");
            let settings_json: String = row.get("settings_json");

            let state = serde_json::from_str::<JobState>(&state_json).map_err(|e| {
                AppErrorView::internal_error(format!("Failed to parse state: {}", e))
            })?;

            let settings = serde_json::from_str::<JobSettings>(&settings_json).map_err(|e| {
                AppErrorView::internal_error(format!("Failed to parse settings: {}", e))
            })?;

            let job =
                Job {
                    id: JobId(uuid::Uuid::parse_str(&id_str).map_err(|e| {
                        AppErrorView::internal_error(format!("Invalid job ID: {}", e))
                    })?),
                    source_path: row.get::<String, _>("source_path").into(),
                    display_name: row.get("display_name"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    created_at: row.get("created_at"),
                    state,
                    settings_snapshot: settings,
                    content_hash: row.get("content_hash"),
                };
            jobs.push(job);
        }

        Ok(jobs)
    }

    /// Get job by ID
    pub async fn get(&self, id: JobId) -> Result<Option<Job>, AppErrorView> {
        let row = sqlx::query(
            "SELECT id, source_path, display_name, size_bytes, content_hash, created_at, state, state_payload, settings_json
             FROM jobs WHERE id = ?"
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to fetch job: {}", e)))?;

        match row {
            Some(row) => {
                let state_json: String = row.get("state_payload");
                let settings_json: String = row.get("settings_json");

                let state = serde_json::from_str::<JobState>(&state_json).map_err(|e| {
                    AppErrorView::internal_error(format!("Failed to parse state: {}", e))
                })?;

                let settings =
                    serde_json::from_str::<JobSettings>(&settings_json).map_err(|e| {
                        AppErrorView::internal_error(format!("Failed to parse settings: {}", e))
                    })?;

                let job = Job {
                    id,
                    source_path: row.get::<String, _>("source_path").into(),
                    display_name: row.get("display_name"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    created_at: row.get("created_at"),
                    state,
                    settings_snapshot: settings,
                    content_hash: row.get("content_hash"),
                };
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Get count of jobs
    pub async fn count(&self) -> Result<u32, AppErrorView> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM jobs")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppErrorView::internal_error(format!("Failed to count jobs: {}", e)))?;

        let count: i64 = row.get("cnt");
        Ok(count as u32)
    }
}

/// Transcript cache and edits repository
pub struct TranscriptRepo {
    pool: SqlitePool,
}

impl TranscriptRepo {
    pub fn new(pool: SqlitePool) -> Self {
        TranscriptRepo { pool }
    }

    /// Store transcript from Groq response
    pub async fn store(
        &self,
        job_id: JobId,
        plain_text: String,
        segments_json: String,
    ) -> Result<(), AppErrorView> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| AppErrorView::internal_error(format!("System time error: {}", e)))?
            .as_millis() as i64;

        sqlx::query(
            "INSERT OR REPLACE INTO transcripts (job_id, plain_text, segments_json, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(job_id.0.to_string())
        .bind(plain_text)
        .bind(segments_json)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to store transcript: {}", e)))?;

        Ok(())
    }

    /// Retrieve transcript
    pub async fn get(&self, job_id: JobId) -> Result<Option<String>, AppErrorView> {
        let row = sqlx::query("SELECT plain_text FROM transcripts WHERE job_id = ?")
            .bind(job_id.0.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to fetch transcript: {}", e))
            })?;

        Ok(row.map(|r| r.get::<String, _>("plain_text")))
    }

    /// Update transcript with user edits
    pub async fn update(&self, job_id: JobId, edited_text: String) -> Result<(), AppErrorView> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| AppErrorView::internal_error(format!("System time error: {}", e)))?
            .as_millis() as i64;

        sqlx::query("UPDATE transcripts SET edited_text = ?, updated_at = ? WHERE job_id = ?")
            .bind(edited_text)
            .bind(now_ms)
            .bind(job_id.0.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to update transcript: {}", e))
            })?;

        Ok(())
    }

    /// Get edited transcript if available
    pub async fn get_edited(&self, job_id: JobId) -> Result<Option<String>, AppErrorView> {
        let row = sqlx::query("SELECT edited_text FROM transcripts WHERE job_id = ?")
            .bind(job_id.0.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to fetch edited transcript: {}", e))
            })?;

        Ok(row.and_then(|r| r.get::<Option<String>, _>("edited_text")))
    }
}

/// Content hash cache for deduplication
pub struct CacheRepo {
    pool: SqlitePool,
}

impl CacheRepo {
    pub fn new(pool: SqlitePool) -> Self {
        CacheRepo { pool }
    }

    /// Store file hash → job_id mapping
    pub async fn store(&self, cache_key: &str, job_id: JobId) -> Result<(), AppErrorView> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| AppErrorView::internal_error(format!("System time error: {}", e)))?
            .as_millis() as i64;

        sqlx::query(
            "INSERT OR REPLACE INTO cache (cache_key, job_id, created_at) VALUES (?, ?, ?)",
        )
        .bind(cache_key)
        .bind(job_id.0.to_string())
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to store cache entry: {}", e)))?;

        Ok(())
    }

    /// Check if file was already processed
    pub async fn get(&self, cache_key: &str) -> Result<Option<JobId>, AppErrorView> {
        let row = sqlx::query("SELECT job_id FROM cache WHERE cache_key = ?")
            .bind(cache_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to fetch cache entry: {}", e))
            })?;

        match row {
            Some(r) => {
                let id_str: String = r.get("job_id");
                let uuid = uuid::Uuid::parse_str(&id_str)
                    .map_err(|e| AppErrorView::internal_error(format!("Invalid job ID: {}", e)))?;
                Ok(Some(JobId(uuid)))
            }
            None => Ok(None),
        }
    }
}

/// Settings repository
pub struct SettingsRepo {
    pool: SqlitePool,
}

impl SettingsRepo {
    pub fn new(pool: SqlitePool) -> Self {
        SettingsRepo { pool }
    }

    /// Store setting
    pub async fn set(&self, key: &str, value: &str) -> Result<(), AppErrorView> {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| AppErrorView::internal_error(format!("Failed to save setting: {}", e)))?;

        Ok(())
    }

    /// Retrieve setting
    pub async fn get(&self, key: &str) -> Result<Option<String>, AppErrorView> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppErrorView::internal_error(format!("Failed to fetch setting: {}", e)))?;

        Ok(row.map(|r| r.get::<String, _>("value")))
    }
}
