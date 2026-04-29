mod adapters;
mod app;
mod core;
mod db;
mod telemetry;
mod types;

use crate::adapters::keyring::KeyringAdapter;
use crate::app::state::SharedState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

#[cfg(feature = "with_tauri")]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle();

            // Initialize database path
            let db_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;

            // Initialize tracing FIRST before anything else
            let log_dir = crate::telemetry::get_log_dir(&db_dir);
            if let Err(e) = crate::telemetry::init_tracing(log_dir) {
                eprintln!("Failed to initialize tracing: {}", e);
            }

            tracing::info!(
                "VideoTranscriber v0.1.0 starting up with app data dir: {}",
                db_dir.display()
            );

            let db_path = db_dir.join("transcriber.db");

            // Create app state asynchronously
            let db_path_clone = db_path.clone();
            let app_handle_clone = app_handle.clone();

            tauri::async_runtime::spawn(async move {
                // Try to load API key from keyring
                let api_key = match KeyringAdapter::get_api_key() {
                    Ok(Some(key)) => {
                        tracing::debug!("API key loaded from keyring");
                        key
                    }
                    Ok(None) => {
                        tracing::warn!("No API key found in keyring. Please save it in settings.");
                        String::new()
                    }
                    Err(e) => {
                        tracing::warn!("Keyring error when loading API key: {}", e);
                        String::new()
                    }
                };

                // Initialize app state
                match crate::app::state::AppState::new(db_path_clone, api_key).await {
                    Ok(state) => {
                        tracing::info!("Application state initialized successfully");

                        // Try to recover active jobs from database
                        recover_active_jobs(&state).await;

                        // Store in Tauri managed state
                        app_handle_clone.manage(Arc::new(RwLock::new(Some(state))));
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize app state: {}", e);
                        // Store None to signal initialization failure
                        app_handle_clone.manage(Arc::new(RwLock::new(None)) as SharedState);
                    }
                }
            });

            // Initialize empty state immediately (will be replaced by async task)
            app.manage(Arc::new(RwLock::new(None)) as SharedState);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Health check
            app::commands::health_check,
            // Queue management
            app::commands::enqueue_files,
            app::commands::list_jobs,
            app::commands::cancel_job,
            app::commands::retry_job,
            app::commands::pause_queue,
            app::commands::resume_queue,
            // Transcript & export
            app::commands::get_transcript,
            app::commands::save_transcript_edit,
            app::commands::export,
            // Settings & secrets
            app::commands::save_api_key,
            app::commands::check_api_key,
            app::commands::delete_api_key,
            app::commands::get_settings,
            app::commands::set_settings,
            // Logging
            app::commands::get_logs,
            app::commands::open_logs_folder,
            // Demo/testing
            app::commands::emit_demo_event,
        ])
        .run({
            let context = tauri::generate_context!();
            context
        })
        .expect("error while running Tauri application");
}

/// Recover active jobs from database on startup
async fn recover_active_jobs(state: &crate::app::state::AppState) {
    #[allow(unused_imports)]
    use crate::types::JobState;

    match state.job_repo.list(None).await {
        Ok(jobs) => {
            let mut recovered = 0;
            for job in jobs {
                // Check if job is in an unfinished state
                let should_recover = matches!(
                    job.state,
                    JobState::Queued
                        | JobState::Probing
                        | JobState::Extracting { .. }
                        | JobState::Chunking { .. }
                        | JobState::Uploading { .. }
                        | JobState::Transcribing { .. }
                        | JobState::Stitching
                        | JobState::Postprocessing
                );

                if should_recover {
                    // Re-enqueue the job
                    state.scheduler.enqueue(job.id).await.ok();
                    recovered += 1;
                }
            }

            if recovered > 0 {
                tracing::info!("Recovered {} active jobs from database", recovered);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to recover active jobs: {}", e);
        }
    }
}

#[cfg(not(feature = "with_tauri"))]
fn main() {
    println!("Tauri feature disabled. Run: cargo run --features with_tauri");
}
