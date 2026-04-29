use crate::adapters::keyring::KeyringAdapter;
use crate::app::state::SharedState;
#[allow(unused)]
use crate::core::cache;
use crate::core::export::{export_transcript as export_transcript_impl, ConflictPolicy};
use crate::types::*;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
#[allow(unused)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

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

    // Load current settings from DB (with defaults as fallback)
    let settings = app_state.get_settings_from_db().await.unwrap_or_default(); // Fallback to defaults if DB unavailable

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
                language: settings.language.clone(),
                output_format: settings.output_format,
                enable_postprocess: settings.enable_postprocess,
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
/// Сначала проверяет отредактированный текст из БД, затем из файла
#[tauri::command]
pub async fn get_transcript(
    state: tauri::State<'_, SharedState>,
    id: JobId,
) -> Result<Transcript, AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    // First try to get edited transcript from database
    if let Ok(Some(edited_text)) = app_state.transcript_repo.get_edited(id).await {
        return Ok(Transcript {
            job_id: id,
            text: edited_text,
        });
    }

    // Then try to load from .txt file
    if let Some(job) = app_state.job_repo.get(id).await? {
        let txt_path = job.source_path.with_extension("txt");
        if txt_path.exists() {
            let text = tokio::fs::read_to_string(&txt_path)
                .await
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
/// Сохраняет в БД и обновляет .txt файл
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

    // Save edited text to database
    if let Some(job) = app_state.job_repo.get(id).await? {
        // Update transcript with edited text
        app_state.transcript_repo.update(id, text.clone()).await?;

        // Also update the .txt file
        let txt_path = job.source_path.with_extension("txt");
        tokio::fs::write(&txt_path, &text)
            .await
            .map_err(|e| AppErrorView::fs_error(format!("Failed to save transcript: {}", e)))?;
    } else {
        return Err(AppErrorView::internal_error("Job not found"));
    }

    Ok(())
}

/// Экспортировать результат в выбранный формат
/// Поддерживает TXT, SRT и JSON
/// Не делает повторных Groq-запросов, использует кэшированные сегменты
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

    // Get transcript text (prefer edited if available)
    let text = if let Ok(Some(edited)) = app_state.transcript_repo.get_edited(id).await {
        edited
    } else if let Ok(Some(plain)) = app_state.transcript_repo.get(id).await {
        plain
    } else {
        return Err(AppErrorView::internal_error(
            "No transcript found for this job",
        ));
    };

    // Get segments from database
    let row = sqlx::query("SELECT segments_json FROM transcripts WHERE job_id = ?")
        .bind(id.0.to_string())
        .fetch_optional(&app_state.transcript_repo.pool)
        .await
        .map_err(|e| AppErrorView::internal_error(format!("Database error: {}", e)))?;

    let segments: Vec<TranscriptSegment> = if let Some(r) = row {
        let segments_json: String = r.get("segments_json");
        serde_json::from_str(&segments_json).unwrap_or_default()
    } else {
        Vec::new()
    };

    let format_str = match format {
        ExportFormat::Txt => "txt",
        ExportFormat::Srt => "srt",
        ExportFormat::Json => "json",
    };

    let base_path = job.source_path.with_extension("");
    let output_path = export_transcript_impl(
        &text,
        &segments,
        format_str,
        &base_path,
        ConflictPolicy::Overwrite,
    )
    .await?;

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

    tracing::info!("API key saved to OS keychain successfully");
    Ok(())
}

/// Проверить наличие сохранённого API ключа
#[tauri::command]
pub async fn check_api_key() -> Result<bool, AppErrorView> {
    KeyringAdapter::has_api_key()
        .map_err(|e| AppErrorView::internal_error(format!("Failed to check API key: {}", e)))
}

/// Удалить API ключ из OS keychain
#[tauri::command]
pub async fn delete_api_key() -> Result<(), AppErrorView> {
    KeyringAdapter::delete_api_key()
        .map_err(|e| AppErrorView::internal_error(format!("Failed to delete API key: {}", e)))
}

/// Получить текущие настройки приложения
#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, SharedState>) -> Result<Settings, AppErrorView> {
    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    let defaults = Settings::default();

    // Try to load each setting from DB, fall back to defaults
    let language = app_state
        .settings_repo
        .get("language")
        .await
        .ok()
        .flatten()
        .unwrap_or(defaults.language);

    let output_format_str = app_state
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

    let parallelism_str = app_state
        .settings_repo
        .get("parallelism")
        .await
        .ok()
        .flatten();
    let parallelism = parallelism_str
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(defaults.parallelism);

    let enable_postprocess_str = app_state
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

/// Обновить настройки приложения
#[tauri::command]
pub async fn set_settings(
    state: tauri::State<'_, SharedState>,
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

    let state_opt = state.read().await;
    let app_state = state_opt
        .as_ref()
        .ok_or_else(|| AppErrorView::internal_error("App not initialized"))?;

    // Save each setting to DB
    app_state
        .settings_repo
        .set("language", &settings.language)
        .await?;

    let format_str = match settings.output_format {
        ExportFormat::Srt => "srt",
        ExportFormat::Json => "json",
        ExportFormat::Txt => "txt",
    };
    app_state
        .settings_repo
        .set("output_format", format_str)
        .await?;

    app_state
        .settings_repo
        .set("parallelism", &settings.parallelism.to_string())
        .await?;

    app_state
        .settings_repo
        .set(
            "enable_postprocess",
            if settings.enable_postprocess {
                "true"
            } else {
                "false"
            },
        )
        .await?;

    tracing::info!(
        "Settings updated: language={}, parallelism={}, enable_postprocess={}",
        settings.language,
        settings.parallelism,
        settings.enable_postprocess
    );
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
// Logging Commands
// ============================================================================

/// Get recent log entries from file
#[tauri::command]
pub async fn get_logs(
    app_handle: tauri::AppHandle,
    lines: Option<u32>,
) -> Result<Vec<String>, AppErrorView> {
    let log_lines = lines.unwrap_or(100).min(1000); // Cap at 1000 lines for performance

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppErrorView::internal_error(format!("Failed to get app data dir: {}", e)))?;

    let log_dir = crate::telemetry::get_log_dir(&app_data_dir);

    // Find the current log file (today's file)
    let mut log_entries = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&log_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                    // Get last N lines
                    let start_idx = if lines.len() > log_lines as usize {
                        lines.len() - log_lines as usize
                    } else {
                        0
                    };
                    log_entries.extend_from_slice(&lines[start_idx..]);
                }
            }
        }
    }

    // Return last N lines across all log files
    let start_idx = if log_entries.len() > log_lines as usize {
        log_entries.len() - log_lines as usize
    } else {
        0
    };

    Ok(log_entries[start_idx..].to_vec())
}

/// Open the logs folder in the system file explorer
#[tauri::command]
pub async fn open_logs_folder(app_handle: tauri::AppHandle) -> Result<(), AppErrorView> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppErrorView::internal_error(format!("Failed to get app data dir: {}", e)))?;

    let log_dir = crate::telemetry::get_log_dir(&app_data_dir);

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| AppErrorView::fs_error(format!("Failed to create log dir: {}", e)))?;

    // Platform-specific folder opening
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("explorer")
            .arg(log_dir.as_os_str())
            .spawn()
            .map_err(|e| AppErrorView::internal_error(format!("Failed to open folder: {}", e)))?;
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg(log_dir.as_os_str())
            .spawn()
            .map_err(|e| AppErrorView::internal_error(format!("Failed to open folder: {}", e)))?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xdg-open")
            .arg(log_dir.as_os_str())
            .spawn()
            .map_err(|e| AppErrorView::internal_error(format!("Failed to open folder: {}", e)))?;
    }

    tracing::info!("Opening logs folder at {}", log_dir.display());

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
