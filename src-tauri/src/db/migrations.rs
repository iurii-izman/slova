// ============================================================================
// Database Migrations
// ============================================================================

use crate::types::AppErrorView;
use sqlx::sqlite::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<(), AppErrorView> {
    // Create jobs table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS jobs (
            id              TEXT PRIMARY KEY,
            source_path     TEXT NOT NULL,
            display_name    TEXT NOT NULL,
            size_bytes      INTEGER NOT NULL,
            content_hash    TEXT,
            created_at      TEXT NOT NULL,
            finished_at     INTEGER,
            state           TEXT NOT NULL,
            state_payload   TEXT NOT NULL,
            output_path     TEXT,
            settings_json   TEXT NOT NULL,
            attempts        INTEGER NOT NULL DEFAULT 0,
            error_message   TEXT,
            error_code      TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppErrorView::internal_error(format!("Failed to create jobs table: {}", e)))?;

    // Create indexes for jobs
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_state ON jobs(state)")
        .execute(pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to create index: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_created ON jobs(created_at DESC)")
        .execute(pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to create index: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_hash ON jobs(content_hash)")
        .execute(pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to create index: {}", e)))?;

    // Create transcripts table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS transcripts (
            job_id          TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
            plain_text      TEXT NOT NULL,
            segments_json   TEXT NOT NULL,
            edited_text     TEXT,
            updated_at      INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        AppErrorView::internal_error(format!("Failed to create transcripts table: {}", e))
    })?;

    // Create cache table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cache (
            cache_key       TEXT PRIMARY KEY,
            job_id          TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
            created_at      INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppErrorView::internal_error(format!("Failed to create cache table: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_cache_job ON cache(job_id)")
        .execute(pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Failed to create index: {}", e)))?;

    // Create settings table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            key             TEXT PRIMARY KEY,
            value           TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppErrorView::internal_error(format!("Failed to create settings table: {}", e)))?;

    Ok(())
}
