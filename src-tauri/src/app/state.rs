use crate::adapters::ffmpeg::FfmpegAdapter;
use crate::adapters::groq::GroqClient;
use crate::core::progress::ProgressBroadcaster;
use crate::core::scheduler::JobScheduler;
use crate::db::{Database, JobRepo};
use crate::types::AppErrorView;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Основное состояние приложения
/// Содержит очередь задач, БД, planировщик и все адаптеры
pub struct AppState {
    pub db: Arc<Database>,
    pub job_repo: Arc<JobRepo>,
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

        // Initialize adapters
        let ffmpeg = Arc::new(FfmpegAdapter::default_new());
        let groq = Arc::new(GroqClient::new(groq_api_key)?);

        // Setup progress broadcast channel
        let (progress_tx, _progress_rx) = mpsc::unbounded_channel();
        let progress = ProgressBroadcaster::new(progress_tx);

        // Create scheduler
        let scheduler = Arc::new(JobScheduler::new(
            ffmpeg.clone(),
            groq.clone(),
            job_repo.clone(),
            progress.clone(),
        ));

        Ok(AppState {
            db,
            job_repo,
            scheduler,
            progress,
            ffmpeg,
            groq,
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
            scheduler: Arc::clone(&self.scheduler),
            progress: self.progress.clone(),
            ffmpeg: Arc::clone(&self.ffmpeg),
            groq: Arc::clone(&self.groq),
        }
    }
}

/// Общее состояние приложения (Arc<RwLock> для безопасной передачи между потоками)
pub type SharedState = Arc<RwLock<Option<AppState>>>;
