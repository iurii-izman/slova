// ============================================================================
// Core Domain Module
// ============================================================================
// Orchestrates the job pipeline:
// - Scheduling (parallelism, backoff)
// - State transitions (state machine)
// - Progress tracking and events
// - Cancellation and pause/resume

pub mod scheduler;
// TODO: pub mod pipeline;
// TODO: pub mod stages;
// TODO: pub mod retry;
// TODO: pub mod cancel;
// TODO: pub mod progress;
