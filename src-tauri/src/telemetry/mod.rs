// ============================================================================
// Telemetry & Logging Module
// ============================================================================
// Structured logging using tracing-subscriber:
// - Console layer (DEBUG/INFO/WARN/ERROR based on RUST_LOG env)
// - Rolling file layer (daily rotation to app data directory)
// - JSON formatting option for parsing
// - Panic hook to log panics
// - Spans with job_id, stage, attempt for context

use std::io;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing subscriber with console and file layers
pub fn init_tracing(log_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)?;

    // File appender with daily rotation
    let file_appender = tracing_appender::rolling::daily(&log_dir, "transcriber.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Console layer (stdout)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(io::stdout)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);

    // File layer (non-blocking)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true);

    // Env filter: read from RUST_LOG, default to INFO
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Combine layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    // Install panic hook
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(
            panic = ?panic_info,
            backtrace = ?std::backtrace::Backtrace::capture(),
            "Application panic"
        );
        default_panic(panic_info);
    }));

    tracing::info!(
        "Logging initialized with console and file layers at {}",
        log_dir.display()
    );

    Ok(())
}

/// Get the configured log directory
pub fn get_log_dir(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("logs")
}
