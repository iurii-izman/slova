use crate::adapters::keyring::KeyringAdapter;
use crate::types::*;
use serde::Serialize;
use serde_json::json;
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
pub async fn health_check() -> Result<HealthStatus, AppErrorView> {
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
    paths: Vec<PathBuf>,
) -> Result<Vec<JobId>, AppErrorView> {
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
        // TODO: validate MP4 video file
    }

    // Generate job IDs (in real implementation, store in DB)
    let ids: Vec<JobId> = paths
        .iter()
        .enumerate()
        .map(|(_i, _p)| JobId::new())
        .collect();

    // Emit sample queue:tick event
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let tick = json!(QueueTick {
        updates: ids
            .iter()
            .enumerate()
            .map(|(_i, id)| JobUpdate {
                id: *id,
                state: JobState::Queued,
                bytes_uploaded: None,
                eta_ms: None,
            })
            .collect(),
        ts: now_ms,
    });

    let _ = app_handle.emit("queue:tick", tick);

    Ok(ids)
}

/// Получить список задач с фильтрацией
#[tauri::command]
pub async fn list_jobs(_filter: Option<JobFilter>) -> Result<Vec<Job>, AppErrorView> {
    // TODO: query from DB with filter
    Ok(vec![])
}

/// Отменить задачу
#[tauri::command]
pub async fn cancel_job(id: JobId) -> Result<(), AppErrorView> {
    // TODO: set cancellation token, emit job:cancelled
    println!("cancel_job called: {}", id);
    Ok(())
}

/// Повторить задачу после ошибки
#[tauri::command]
pub async fn retry_job(id: JobId) -> Result<(), AppErrorView> {
    // TODO: reset state to Queued, increment attempts counter
    println!("retry_job called: {}", id);
    Ok(())
}

/// Поставить всю очередь на паузу
#[tauri::command]
pub async fn pause_queue() -> Result<(), AppErrorView> {
    // TODO: pause scheduler
    println!("pause_queue called");
    Ok(())
}

/// Возобновить обработку очереди
#[tauri::command]
pub async fn resume_queue() -> Result<(), AppErrorView> {
    // TODO: resume scheduler
    println!("resume_queue called");
    Ok(())
}

// ============================================================================
// Transcript & Export Commands
// ============================================================================

/// Получить текст транскрипции для задачи
#[tauri::command]
pub async fn get_transcript(id: JobId) -> Result<Transcript, AppErrorView> {
    // TODO: load from disk or DB
    Ok(Transcript {
        job_id: id,
        text: String::new(),
    })
}

/// Сохранить отредактированный текст транскрипции
#[tauri::command]
pub async fn save_transcript_edit(id: JobId, text: String) -> Result<(), AppErrorView> {
    if text.is_empty() {
        return Err(AppErrorView::new(
            "INVALID_INPUT",
            "Transcript text cannot be empty",
        ));
    }

    // TODO: save edited transcript to disk/DB
    println!("save_transcript_edit called for {}", id);
    Ok(())
}

/// Экспортировать результат в выбранный формат
#[tauri::command]
pub async fn export(
    _app_handle: tauri::AppHandle,
    id: JobId,
    format: ExportFormat,
) -> Result<PathBuf, AppErrorView> {
    // TODO: generate file in requested format, return actual path
    let ext = match format {
        ExportFormat::Txt => "txt",
        ExportFormat::Srt => "srt",
        ExportFormat::Json => "json",
    };

    let path = PathBuf::from(format!("C:/Users/you/Documents/{}.{}", id, ext));
    Ok(path)
}

// ============================================================================
// Settings & Secrets Commands
// ============================================================================

/// Сохранить API ключ в OS keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service)
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

    // Store securely in OS keychain
    KeyringAdapter::save_api_key(&key)?;

    println!("API key saved to OS keychain successfully");
    Ok(())
}

/// Получить текущие настройки приложения
#[tauri::command]
pub async fn get_settings() -> Result<Settings, AppErrorView> {
    // TODO: load from SQLite settings repo
    // For now return defaults
    Ok(Settings::default())
}

/// Обновить настройки приложения
#[tauri::command]
pub async fn set_settings(settings: Settings) -> Result<(), AppErrorView> {
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
        ts: ts,
    });

    app_handle
        .emit("queue:tick", sample)
        .map_err(|e| AppErrorView::internal_error(format!("emit error: {}", e)))?;

    Ok(())
}
