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
        .setup(|app| {
            let app_handle = app.handle();

            // Initialize database path
            let db_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;

            let db_path = db_dir.join("transcriber.db");

            // Create app state asynchronously
            let db_path_clone = db_path.clone();
            let app_handle_clone = app_handle.clone();

            tauri::async_runtime::spawn(async move {
                // Try to load API key from keyring
                let api_key = match KeyringAdapter::get_api_key() {
                    Ok(Some(key)) => key,
                    Ok(None) => {
                        eprintln!("No API key found in keyring. Please save it first.");
                        String::new()
                    }
                    Err(e) => {
                        eprintln!("Keyring error: {}. Please save API key first.", e);
                        String::new()
                    }
                };

                // Initialize app state
                match crate::app::state::AppState::new(db_path_clone, api_key).await {
                    Ok(state) => {
                        // Store in Tauri managed state
                        app_handle_clone.manage(Arc::new(RwLock::new(Some(state))));
                        println!("VideoTranscriber v0.1.0 initialized successfully");
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize app state: {}", e);
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
            app::commands::get_settings,
            app::commands::set_settings,
            // Demo/testing
            app::commands::emit_demo_event,
        ])
        .run({
            let context = tauri::generate_context!();
            context
        })
        .expect("error while running Tauri application");
}

#[cfg(not(feature = "with_tauri"))]
fn main() {
    println!("Tauri feature disabled. Run: cargo run --features with_tauri");
}
