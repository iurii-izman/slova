// ============================================================================
// Core Domain Module
// ============================================================================
// Orchestrates the job pipeline:
// - Scheduling (parallelism, backoff)
// - State transitions (state machine)
// - Progress tracking and events
// - Cancellation and pause/resume

pub mod cancellation;
pub mod pipeline;
pub mod progress;
pub mod retry;
pub mod scheduler;
pub mod stages;
