use std::sync::Arc;
use tokio::sync::RwLock;

/// Основное состояние приложения
/// Содержит очередь задач, БД подключение, конфиг, и планировщик
pub struct AppState {
    // TODO: when implemented
    // pub db: sqlx::PgPool,
    // pub scheduler: Arc<JobScheduler>,
    // pub config: AppConfig,
    // pub auth: KeyringAdapter,
    // pub ffmpeg: FfmpegAdapter,
    // pub groq: GroqClient,
}

impl AppState {
    pub fn new() -> Self {
        AppState {}
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Общее состояние приложения (Arc<RwLock> для безопасной передачи между потоками)
pub type SharedState = Arc<RwLock<AppState>>;
