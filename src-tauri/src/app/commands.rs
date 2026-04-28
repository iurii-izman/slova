use crate::adapters::keyring::KeyringAdapter;
use crate::app::state::SharedState;
use crate::types::*;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;

#[derive(Serialize)]
pub struct HealthStatus {
    pub ok: bool,
    pub version: &'static str,
}

/// Health check для проверки соединения с backend'ом
#[tauri::command]
pub async fn health_check(
    state: tauri::State<'_, SharedState>,
) -> Result<HealthStatus, AppErrorView> {
    let state_opt = state.read().await;
    if let Some(app_state) = state_opt.as_ref() {
        app_state.health_check().await?;
    }
    Ok(HealthStatus {
        ok: true,
        version: "0.1.0",
    })
}

// ============================================================================
// Queue Management Commands
// ============================================================================

/// Добавить файлы в очередь обработки
#[tauri::command]
pub async fn enqueue_files(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    paths: Vec<PathBuf>,
) -> Result<Vec<JobId>, AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    if paths.is_empty() {
        return Err(AppErrorView::new("INVALID_INPUT", "No files provided"));
    }

    // Validate paths
    for path in &paths {
        if !path.exists() {
            return Err(AppErrorView::invalid_file(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Check if it's a valid MP4
        if path.extension().and_then(|s| s.to_str()) != Some("mp4") {
            return Err(AppErrorView::invalid_file(format!(
                "File must be MP4: {}",
                path.display()
            )));
        }
    }

    // Create jobs and enqueue them
    let mut job_ids = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for path in paths {
        let job_id = JobId::new();
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_size = std::fs::metadata(&path)
            .map_err(|e| AppErrorView::fs_error(format!("Failed to stat file: {}", e)))?
            .len();

        // Calculate content hash for deduplication
        let hash = calculate_file_hash(&path).await.ok();

        let job = Job {
            id: job_id,
            source_path: path,
            display_name,
            size_bytes: file_size,
            created_at: now.clone(),
            state: JobState::Queued,
            settings_snapshot: JobSettings {
                language: "ru".into(),
                output_format: ExportFormat::Txt,
            },
            content_hash: hash,
        };

        // Store in DB
        app_state.job_repo.insert(&job).await?;

        // Enqueue in scheduler
        app_state.scheduler.enqueue(job_id).await?;

        job_ids.push(job_id);
    }

    // Emit initial state
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let tick = json!(QueueTick {
        updates: job_ids
            .iter()
            .map(|id| JobUpdate {
                id: *id,
                state: JobState::Queued,
                bytes_uploaded: None,
                eta_ms: None,
            })
            .collect(),
        ts: now_ms,
    });

    let _ = app_handle.emit("queue:tick", tick);

    Ok(job_ids)
}

/// Получить список задач с фильтрацией
#[tauri::command]
pub async fn list_jobs(
    state: tauri::State<'_, SharedState>,
    filter: Option<JobFilter>,
) -> Result<Vec<Job>, AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    app_state.job_repo.list(filter).await
}

/// Отменить задачу
#[tauri::command]
pub async fn cancel_job(
    state: tauri::State<'_, SharedState>,
    id: JobId,
) -> Result<(), AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    app_state.scheduler.cancel(id);
    Ok(())
}

/// Повторить задачу после ошибки
#[tauri::command]
pub async fn retry_job(
    state: tauri::State<'_, SharedState>,
    id: JobId,
) -> Result<(), AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    // Get job from DB, reset state to Queued
    if let Some(mut job) = app_state.job_repo.get(id).await? {
        job.state = JobState::Queued;
        app_state.job_repo.update_state(id, &job.state).await?;
        app_state.scheduler.enqueue(id).await?;
    }

    Ok(())
}

/// Поставить всю очередь на паузу
#[tauri::command]
pub async fn pause_queue(state: tauri::State<'_, SharedState>) -> Result<(), AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    app_state.scheduler.pause();
    Ok(())
}

/// Возобновить обработку очереди
#[tauri::command]
pub async fn resume_queue(state: tauri::State<'_, SharedState>) -> Result<(), AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    app_state.scheduler.resume();
    Ok(())
}

// ============================================================================
// Transcript & Export Commands
// ============================================================================

/// Получить текст транскрипции для задачи
#[tauri::command]
pub async fn get_transcript(
    state: tauri::State<'_, SharedState>,
    id: JobId,
) -> Result<Transcript, AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    // Try to load from .txt file (once written)
    if let Some(job) = app_state.job_repo.get(id).await? {
        let txt_path = job.source_path.with_extension("txt");
        if txt_path.exists() {
            let text = std::fs::read_to_string(&txt_path)
                .map_err(|e| AppErrorView::fs_error(format!("Failed to read transcript: {}", e)))?;
            return Ok(Transcript { job_id: id, text });
        }
    }

    // Return empty if not yet written
    Ok(Transcript {
        job_id: id,
        text: String::new(),
    })
}

/// Сохранить отредактированный текст транскрипции
#[tauri::command]
pub async fn save_transcript_edit(
    state: tauri::State<'_, SharedState>,
    id: JobId,
    text: String,
) -> Result<(), AppErrorView> {
    if text.is_empty() {
        return Err(AppErrorView::new(
            "INVALID_INPUT",
            "Transcript text cannot be empty",
        ));
    }

    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    // Save to database
    app_state
        .job_repo
        .update_state(
            id,
            &JobState::Done {
                output_path: Default::default(),
                duration_ms: 0,
            },
        )
        .await?;

    // Also save edited text to .txt file
    if let Some(job) = app_state.job_repo.get(id).await? {
        let txt_path = job.source_path.with_extension("txt");
        std::fs::write(&txt_path, &text)
            .map_err(|e| AppErrorView::fs_error(format!("Failed to save transcript: {}", e)))?;
    }

    Ok(())
}

/// Экспортировать результат в выбранный формат
#[tauri::command]
pub async fn export(
    state: tauri::State<'_, SharedState>,
    id: JobId,
    format: ExportFormat,
) -> Result<PathBuf, AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    let job = app_state
        .job_repo
        .get(id)
        .await?
        .ok_or_else(|| AppErrorView::internal_error("Job not found"))?;

    let ext = match format {
        ExportFormat::Txt => "txt",
        ExportFormat::Srt => "srt",
        ExportFormat::Json => "json",
    };

    let output_path = job.source_path.with_extension(ext);
    Ok(output_path)
}

// ============================================================================
// Settings & Secrets Commands
// ============================================================================

/// Сохранить API ключ в OS keychain
#[tauri::command]
pub async fn save_api_key(key: String) -> Result<(), AppErrorView> {
    if key.is_empty() {
        return Err(AppErrorView::new(
            "INVALID_INPUT",
            "API key cannot be empty",
        ));
    }

    // Validate basic structure (Groq API keys are typically 40+ chars)
    if key.len() < 20 {
        return Err(AppErrorView::new(
            "INVALID_INPUT",
            "API key appears too short",
        ));
    }

    // Store securely in OS keyring
    KeyringAdapter::save_api_key(&key)?;

    println!("API key saved to OS keychain successfully");
    Ok(())
}

/// Получить текущие настройки приложения
#[tauri::command]
pub async fn get_settings(_state: tauri::State<'_, SharedState>) -> Result<Settings, AppErrorView> {
    // TODO: load from SQLite settings repo
    // For now return defaults
    Ok(Settings::default())
}

/// Обновить настройки приложения
#[tauri::command]
pub async fn set_settings(
    _state: tauri::State<'_, SharedState>,
    settings: Settings,
) -> Result<(), AppErrorView> {
    if settings.parallelism == 0 {
        return Err(AppErrorView::new(
            "INVALID_INPUT",
            "Parallelism must be > 0",
        ));
    }

    if settings.parallelism > 10 {
        return Err(AppErrorView::new(
            "INVALID_INPUT",
            "Parallelism should not exceed 10",
        ));
    }

    // TODO: save to SQLite settings repo
    println!("Settings updated: parallelism={}", settings.parallelism);
    Ok(())
}

// ============================================================================
// Demo/Testing Commands
// ============================================================================

/// Helper: emit demo events during development
#[tauri::command]
pub async fn emit_demo_event(app_handle: tauri::AppHandle) -> Result<(), AppErrorView> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let sample = json!(QueueTick {
        updates: vec![],
        ts,
    });

    app_handle
        .emit("queue:tick", sample)
        .map_err(|e| AppErrorView::internal_error(format!("emit error: {}", e)))?;

    Ok(())
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate SHA256 hash of a file (for deduplication)
async fn calculate_file_hash(path: &PathBuf) -> Result<String, AppErrorView> {
    let mut hasher = Sha256::new();
    let data = tokio::fs::read(path)
        .await
        .map_err(|e| AppErrorView::fs_error(format!("Failed to read file: {}", e)))?;
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}
