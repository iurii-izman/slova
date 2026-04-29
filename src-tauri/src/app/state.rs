use crate::adapters::ffmpeg::FfmpegAdapter;
use crate::adapters::groq::GroqClient;
use crate::core::progress::ProgressBroadcaster;
use crate::core::scheduler::JobScheduler;
use crate::db::{CacheRepo, Database, JobRepo, SettingsRepo, TranscriptRepo};
use crate::types::{AppErrorView, ExportFormat, Settings};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Основное состояние приложения
/// Содержит очередь задач, БД, планировщик и все адаптеры
pub struct AppState {
    pub db: Arc<Database>,
    pub job_repo: Arc<JobRepo>,
    pub transcript_repo: Arc<TranscriptRepo>,
    pub cache_repo: Arc<CacheRepo>,
    pub settings_repo: Arc<SettingsRepo>,
    pub scheduler: Arc<JobScheduler>,
    pub progress: ProgressBroadcaster,
    pub ffmpeg: Arc<FfmpegAdapter>,
    pub groq: Arc<GroqClient>,
}

impl AppState {
    pub async fn new(
        db_path: std::path::PathBuf,
        groq_api_key: String,
    ) -> Result<Self, AppErrorView> {
        // Initialize database
        let db = Arc::new(Database::init(&db_path).await?);
        let job_repo = Arc::new(JobRepo::new(db.pool.clone()));
        let transcript_repo = Arc::new(TranscriptRepo::new(db.pool.clone()));
        let cache_repo = Arc::new(CacheRepo::new(db.pool.clone()));
        let settings_repo = Arc::new(SettingsRepo::new(db.pool.clone()));

        // Initialize adapters
        let ffmpeg = Arc::new(FfmpegAdapter::default_new());
        let groq = Arc::new(GroqClient::new(groq_api_key)?);

        // Setup progress broadcast channel
        let (progress_tx, _progress_rx) = mpsc::unbounded_channel();
        let progress = ProgressBroadcaster::new(progress_tx);

        // Create scheduler with postprocessing model
        // Default model: llama-3.1-8b-instant (can be overridden in Settings)
        let postprocess_model = "llama-3.1-8b-instant".to_string();
        let scheduler = Arc::new(JobScheduler::new(
            ffmpeg.clone(),
            groq.clone(),
            job_repo.clone(),
            progress.clone(),
            postprocess_model,
        ));

        Ok(AppState {
            db,
            job_repo,
            transcript_repo,
            cache_repo,
            settings_repo,
            scheduler,
            progress,
            ffmpeg,
            groq,
        })
    }

    /// Load settings from DB with defaults as fallback
    pub async fn get_settings_from_db(&self) -> Result<Settings, AppErrorView> {
        let defaults = Settings::default();

        // Try to load each setting from DB, fall back to defaults
        let language = self
            .settings_repo
            .get("language")
            .await
            .ok()
            .flatten()
            .unwrap_or(defaults.language);

        let output_format_str = self
            .settings_repo
            .get("output_format")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "txt".to_string());
        let output_format = match output_format_str.as_str() {
            "srt" => ExportFormat::Srt,
            "json" => ExportFormat::Json,
            _ => ExportFormat::Txt,
        };

        let parallelism_str = self.settings_repo.get("parallelism").await.ok().flatten();
        let parallelism = parallelism_str
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(defaults.parallelism);

        let enable_postprocess_str = self
            .settings_repo
            .get("enable_postprocess")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "false".to_string());
        let enable_postprocess = enable_postprocess_str == "true";

        Ok(Settings {
            language,
            output_format,
            parallelism,
            enable_postprocess,
            groq_model: defaults.groq_model,
        })
    }

    /// Verify all systems are healthy
    pub async fn health_check(&self) -> Result<(), AppErrorView> {
        self.db.health_check().await?;
        Ok(())
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        AppState {
            db: Arc::clone(&self.db),
            job_repo: Arc::clone(&self.job_repo),
            transcript_repo: Arc::clone(&self.transcript_repo),
            cache_repo: Arc::clone(&self.cache_repo),
            settings_repo: Arc::clone(&self.settings_repo),
            scheduler: Arc::clone(&self.scheduler),
            progress: self.progress.clone(),
            ffmpeg: Arc::clone(&self.ffmpeg),
            groq: Arc::clone(&self.groq),
        }
    }
}

/// Общее состояние приложения (Arc<RwLock> для безопасной передачи между потоками)
pub type SharedState = Arc<RwLock<Option<AppState>>>;
