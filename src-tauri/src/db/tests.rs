// ============================================================================
// Database Tests
// ============================================================================

#[cfg(test)]
mod repository_tests {
    use crate::db::{CacheRepo, JobRepo, SettingsRepo, TranscriptRepo};
    use crate::types::{ExportFormat, Job, JobId, JobSettings, JobState};
    use std::path::PathBuf;

    // Helper to create in-memory SQLite pool for testing
    async fn setup_test_db() -> sqlx::SqlitePool {
        use sqlx::sqlite::SqlitePoolOptions;

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        // Run migrations
        super::super::migrations::run(&pool)
            .await
            .expect("Failed to run migrations");

        // Disable FOREIGN KEY constraints for testing (to avoid references issues)
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&pool)
            .await
            .expect("Failed to disable foreign keys");

        pool
    }

    // Helper to create a test job
    fn create_test_job(idx: usize) -> Job {
        Job {
            id: JobId::new(),
            source_path: PathBuf::from(format!("/test/video{}.mp4", idx)),
            display_name: format!("test_video{}.mp4", idx),
            size_bytes: 1024 * (idx + 1) as u64,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            state: JobState::Queued,
            settings_snapshot: JobSettings {
                language: "ru".to_string(),
                output_format: ExportFormat::Txt,
            },
            content_hash: Some(format!("hash_{}", idx)),
        }
    }

    #[tokio::test]
    async fn test_job_repo_insert_and_get() {
        let pool = setup_test_db().await;
        let repo = JobRepo::new(pool);

        let job = create_test_job(0);
        let job_id = job.id;

        // Insert
        repo.insert(&job).await.expect("Failed to insert job");

        // Get
        let fetched = repo
            .get(job_id)
            .await
            .expect("Failed to fetch job")
            .expect("Job not found");

        assert_eq!(fetched.id, job.id);
        assert_eq!(fetched.display_name, job.display_name);
        assert_eq!(fetched.size_bytes, job.size_bytes);
    }

    #[tokio::test]
    async fn test_job_repo_list() {
        let pool = setup_test_db().await;
        let repo = JobRepo::new(pool);

        // Insert multiple jobs
        for i in 0..3 {
            let job = create_test_job(i);
            repo.insert(&job).await.expect("Failed to insert job");
        }

        // List
        let jobs = repo.list(None).await.expect("Failed to list jobs");

        assert_eq!(jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_job_repo_update_state() {
        let pool = setup_test_db().await;
        let repo = JobRepo::new(pool);

        let job = create_test_job(0);
        let job_id = job.id;

        repo.insert(&job).await.expect("Failed to insert job");

        // Update state
        let new_state = JobState::Probing;
        repo.update_state(job_id, &new_state)
            .await
            .expect("Failed to update state");

        // Verify
        let fetched = repo
            .get(job_id)
            .await
            .expect("Failed to fetch job")
            .expect("Job not found");

        match fetched.state {
            JobState::Probing => {}
            _ => panic!("State was not updated correctly"),
        }
    }

    #[tokio::test]
    async fn test_job_repo_count() {
        let pool = setup_test_db().await;
        let repo = JobRepo::new(pool);

        assert_eq!(repo.count().await.unwrap(), 0);

        // Insert jobs
        for i in 0..5 {
            let job = create_test_job(i);
            repo.insert(&job).await.expect("Failed to insert job");
        }

        assert_eq!(repo.count().await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_transcript_repo() {
        let pool = setup_test_db().await;
        let job_repo = JobRepo::new(pool.clone());
        let transcript_repo = TranscriptRepo::new(pool);

        // Create job first (for FOREIGN KEY)
        let job = create_test_job(0);
        let job_id = job.id;
        job_repo.insert(&job).await.expect("Failed to insert job");

        let text = "Test transcript text".to_string();
        let segments = r#"[{"start_ms": 0, "end_ms": 1000, "text": "Test"}]"#.to_string();

        // Store
        transcript_repo
            .store(job_id, text.clone(), segments)
            .await
            .expect("Failed to store transcript");

        // Get
        let fetched = transcript_repo
            .get(job_id)
            .await
            .expect("Failed to fetch transcript")
            .expect("Transcript not found");

        assert_eq!(fetched, text);
    }

    #[tokio::test]
    async fn test_transcript_repo_edit() {
        let pool = setup_test_db().await;
        let job_repo = JobRepo::new(pool.clone());
        let transcript_repo = TranscriptRepo::new(pool);

        // Create job first (for FOREIGN KEY)
        let job = create_test_job(0);
        let job_id = job.id;
        job_repo.insert(&job).await.expect("Failed to insert job");

        let original_text = "Original text".to_string();
        let segments = r#"[{"start_ms": 0, "end_ms": 1000, "text": "Test"}]"#.to_string();

        transcript_repo
            .store(job_id, original_text, segments)
            .await
            .expect("Failed to store transcript");

        // Edit
        let edited_text = "Edited text".to_string();
        transcript_repo
            .update(job_id, edited_text.clone())
            .await
            .expect("Failed to update transcript");

        // Get edited version
        let fetched = transcript_repo
            .get_edited(job_id)
            .await
            .expect("Failed to fetch edited transcript")
            .expect("Edited transcript not found");

        assert_eq!(fetched, edited_text);
    }

    #[tokio::test]
    async fn test_cache_repo() {
        let pool = setup_test_db().await;
        let job_repo = JobRepo::new(pool.clone());
        let cache_repo = CacheRepo::new(pool);

        // Create job first (for FOREIGN KEY)
        let job = create_test_job(0);
        let job_id = job.id;
        job_repo.insert(&job).await.expect("Failed to insert job");

        let cache_key = "sha256_abc123def456";

        // Store
        cache_repo
            .store(cache_key, job_id)
            .await
            .expect("Failed to store cache entry");

        // Get
        let fetched = cache_repo
            .get(cache_key)
            .await
            .expect("Failed to fetch cache entry")
            .expect("Cache entry not found");

        assert_eq!(fetched, job_id);
    }

    #[tokio::test]
    async fn test_settings_repo() {
        let pool = setup_test_db().await;
        let repo = SettingsRepo::new(pool);

        let key = "language";
        let value = "ru";

        // Set
        repo.set(key, value).await.expect("Failed to set setting");

        // Get
        let fetched = repo
            .get(key)
            .await
            .expect("Failed to fetch setting")
            .expect("Setting not found");

        assert_eq!(fetched, value);
    }
}
