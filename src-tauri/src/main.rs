mod adapters;
mod app;
mod core;
mod db;
mod telemetry;
mod types;

#[cfg(feature = "with_tauri")]
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            println!("Starting VideoTranscriber v0.1.0");

            // Initialize database
            let db_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;

            let db_path = db_dir.join("transcriber.db");

            // TODO: Pass database pool to app state
            // For now, just log that we know where it would go
            println!("Database will be at: {}", db_path.display());

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
        // TODO: fix icon generation in build.rs, then use generate_context!()
        // For now, skip icon bundling in dev
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
