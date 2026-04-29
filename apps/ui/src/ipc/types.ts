// ============================================================================
// NOTE: Ideally generated from Rust via specta/tauri-specta
// For now, manually maintained. TODO: migrate to specta when compatible
// ============================================================================

// ============================================================================
// Job Identity & State
// ============================================================================

export type JobId = string; // UUID as string

export type JobState =
  | { kind: "Queued" }
  | { kind: "Probing" }
  | { kind: "Extracting"; data: { progress: number } }
  | { kind: "Chunking"; data: { progress: number } }
  | { kind: "Uploading"; data: { progress: number; chunk_idx: number; chunk_total: number } }
  | { kind: "Transcribing"; data: { chunk_idx: number; chunk_total: number } }
  | { kind: "Stitching" }
  | { kind: "Postprocessing" }
  | { kind: "Done"; data: { output_path: string; duration_ms: number } }
  | { kind: "Failed"; data: { error: AppErrorView; attempts: number } }
  | { kind: "Cancelled" }
  | { kind: "Paused" };

export type Job = {
  id: JobId;
  source_path: string; // PathBuf → string
  display_name: string;
  size_bytes: number;
  created_at: string; // ISO 8601
  state: JobState;
  settings_snapshot: JobSettings;
  content_hash: string | null; // SHA256 hex
};

// ============================================================================
// Settings & Configuration
// ============================================================================

export type ExportFormat = "txt" | "srt" | "json"; // lowercase

export type Settings = {
  language: string;
  output_format: ExportFormat;
  parallelism: number;
  enable_postprocess: boolean;
  groq_model: string;
};

export type JobSettings = {
  language: string;
  output_format: ExportFormat;
  enable_postprocess: boolean;
};

export type JobFilter = {
  state?: string; // filter by state kind
  limit?: number;
  offset?: number;
};

// ============================================================================
// Transcript & Export
// ============================================================================

export type Transcript = {
  job_id: JobId;
  text: string;
};

export type TranscriptSegment = {
  start_ms: number;
  end_ms: number;
  text: string;
};

// ============================================================================
// Error Handling
// ============================================================================

export type AppErrorView = {
  code: string; // e.g. "INVALID_FILE", "RATE_LIMIT", "AUTH_FAILED"
  message: string;
  details?: string;
};

export const ErrorCodes = {
  INVALID_FILE: "INVALID_FILE",
  INVALID_INPUT: "INVALID_INPUT",
  RATE_LIMIT: "RATE_LIMIT",
  AUTH_FAILED: "AUTH_FAILED",
  NETWORK_ERROR: "NETWORK_ERROR",
  FS_ERROR: "FS_ERROR",
  INTERNAL_ERROR: "INTERNAL_ERROR",
} as const;

// ============================================================================
// Events (emitted from backend)
// ============================================================================

export type QueueTick = {
  updates: JobUpdate[];
  ts: number; // milliseconds since epoch
};

export type JobUpdate = {
  id: JobId;
  state: JobState;
  bytes_uploaded?: number;
  eta_ms?: number;
};

export type RateLimitEvent = {
  retry_after_ms: number;
};

export type AppErrorEvent = {
  error: AppErrorView;
  job_id?: JobId;
};

// Union of all app events
export type AppEvent =
  | { type: "queue:tick"; payload: QueueTick }
  | { type: "job:done"; payload: { id: JobId; state: JobState } }
  | { type: "job:failed"; payload: { id: JobId; state: JobState } }
  | { type: "job:cancelled"; payload: { id: JobId; state: JobState } }
  | { type: "queue:idle"; payload: null }
  | { type: "app:error"; payload: AppErrorEvent }
  | { type: "app:rate-limited"; payload: RateLimitEvent }
  | { type: "app:auth-failed"; payload: null };

export type AppEventListener = (ev: AppEvent) => void;
