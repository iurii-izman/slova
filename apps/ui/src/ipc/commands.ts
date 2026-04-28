import { invoke } from "@tauri-apps/api/tauri";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Job,
  JobId,
  JobFilter,
  Settings,
  Transcript,
  ExportFormat,
  AppErrorView,
  AppEvent,
  AppEventListener,
  QueueTick,
  AppErrorEvent,
  RateLimitEvent,
} from "./types";

// ============================================================================
// Health Check
// ============================================================================

export async function healthCheck(): Promise<{ ok: boolean; version: string }> {
  return await invoke("health_check");
}

// ============================================================================
// Queue Management Commands
// ============================================================================

/**
 * Добавить видеофайлы в очередь обработки.
 * Возвращает список ID созданных задач.
 */
export async function enqueueFiles(paths: string[]): Promise<JobId[]> {
  return await invoke("enqueue_files", { paths });
}

/**
 * Получить список задач с опциональной фильтрацией.
 */
export async function listJobs(filter?: JobFilter): Promise<Job[]> {
  return await invoke("list_jobs", { filter });
}

/**
 * Отменить задачу по ID.
 */
export async function cancelJob(id: JobId): Promise<void> {
  return await invoke("cancel_job", { id });
}

/**
 * Повторить задачу после ошибки.
 */
export async function retryJob(id: JobId): Promise<void> {
  return await invoke("retry_job", { id });
}

/**
 * Поставить всю очередь на паузу.
 */
export async function pauseQueue(): Promise<void> {
  return await invoke("pause_queue");
}

/**
 * Возобновить обработку очереди.
 */
export async function resumeQueue(): Promise<void> {
  return await invoke("resume_queue");
}

// ============================================================================
// Transcript & Export Commands
// ============================================================================

/**
 * Получить текст транскрипции для задачи.
 */
export async function getTranscript(id: JobId): Promise<Transcript> {
  return await invoke("get_transcript", { id });
}

/**
 * Сохранить отредактированный текст транскрипции.
 */
export async function saveTranscriptEdit(id: JobId, text: string): Promise<void> {
  return await invoke("save_transcript_edit", { id, text });
}

/**
 * Экспортировать результат в выбранный формат.
 * Возвращает путь к созданному файлу.
 */
export async function exportJob(id: JobId, format: ExportFormat): Promise<string> {
  return await invoke("export", { id, format });
}

// ============================================================================
// Settings & Secrets Commands
// ============================================================================

/**
 * Сохранить API ключ в OS keychain.
 */
export async function saveApiKey(key: string): Promise<void> {
  return await invoke("save_api_key", { key });
}

/**
 * Получить текущие настройки приложения.
 */
export async function getSettings(): Promise<Settings> {
  return await invoke("get_settings");
}

/**
 * Обновить настройки приложения.
 */
export async function setSettings(settings: Settings): Promise<void> {
  return await invoke("set_settings", { settings });
}

// ============================================================================
// Demo/Testing Commands
// ============================================================================

/**
 * Helper: emit demo event during development.
 */
export async function emitDemoEvent(): Promise<void> {
  return await invoke("emit_demo_event");
}

// ============================================================================
// Event Bus
// ============================================================================

type EventName =
  | "queue:tick"
  | "job:done"
  | "job:failed"
  | "job:cancelled"
  | "queue:idle"
  | "app:error"
  | "app:rate-limited"
  | "app:auth-failed";

const eventNames: EventName[] = [
  "queue:tick",
  "job:done",
  "job:failed",
  "job:cancelled",
  "queue:idle",
  "app:error",
  "app:rate-limited",
  "app:auth-failed",
];

/**
 * Subscribe to all app events.
 * Returns a function to unsubscribe from all listeners.
 */
export function onAppEvent(callback: AppEventListener): () => Promise<void> {
  const unlisteners: Promise<UnlistenFn>[] = [];

  for (const eventName of eventNames) {
    const promise = listen<any>(eventName, (event) => {
      // Normalize event payload to AppEvent format
      const appEvent = normalizeEvent(eventName, event.payload);
      callback(appEvent);
    });
    unlisteners.push(promise);
  }

  // Return cleanup function
  return async () => {
    const fns = await Promise.all(unlisteners);
    fns.forEach((fn) => fn());
  };
}

/**
 * Helper to convert Tauri event to AppEvent format.
 */
function normalizeEvent(eventName: EventName, payload: any): AppEvent {
  switch (eventName) {
    case "queue:tick":
      return { type: "queue:tick", payload: payload as QueueTick };

    case "job:done":
      return { type: "job:done", payload };

    case "job:failed":
      return { type: "job:failed", payload };

    case "job:cancelled":
      return { type: "job:cancelled", payload };

    case "queue:idle":
      return { type: "queue:idle", payload: null };

    case "app:error":
      return { type: "app:error", payload: payload as AppErrorEvent };

    case "app:rate-limited":
      return { type: "app:rate-limited", payload: payload as RateLimitEvent };

    case "app:auth-failed":
      return { type: "app:auth-failed", payload: null };

    default:
      const _exhaustive: never = eventName;
      throw new Error(`Unknown event: ${_exhaustive}`);
  }
}

// ============================================================================
// Helper: Per-event subscriptions
// ============================================================================

export async function onQueueTick(callback: (tick: QueueTick) => void): Promise<UnlistenFn> {
  return listen<any>("queue:tick", (event) => {
    callback(event.payload as QueueTick);
  });
}

export async function onJobDone(callback: (id: JobId, state: any) => void): Promise<UnlistenFn> {
  return listen<any>("job:done", (event) => {
    const payload = event.payload as any;
    callback(payload.id, payload.state);
  });
}

export async function onJobFailed(callback: (id: JobId, state: any) => void): Promise<UnlistenFn> {
  return listen<any>("job:failed", (event) => {
    const payload = event.payload as any;
    callback(payload.id, payload.state);
  });
}

export async function onJobCancelled(
  callback: (id: JobId, state: any) => void,
): Promise<UnlistenFn> {
  return listen<any>("job:cancelled", (event) => {
    const payload = event.payload as any;
    callback(payload.id, payload.state);
  });
}

export async function onQueueIdle(callback: () => void): Promise<UnlistenFn> {
  return listen("queue:idle", () => {
    callback();
  });
}

export async function onAppError(callback: (error: AppErrorEvent) => void): Promise<UnlistenFn> {
  return listen<any>("app:error", (event) => {
    callback(event.payload as AppErrorEvent);
  });
}

export async function onRateLimit(callback: (event: RateLimitEvent) => void): Promise<UnlistenFn> {
  return listen<any>("app:rate-limited", (event) => {
    callback(event.payload as RateLimitEvent);
  });
}

export async function onAuthFailed(callback: () => void): Promise<UnlistenFn> {
  return listen("app:auth-failed", () => {
    callback();
  });
}
