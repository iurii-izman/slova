// ============================================================================
// Progress Tracking & Event Broadcasting
// ============================================================================

use crate::types::{JobId, JobState, JobUpdate, QueueTick};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Progress event for a job
#[derive(Clone, Debug)]
pub struct ProgressEvent {
    pub job_id: JobId,
    pub state: JobState,
    pub bytes_uploaded: Option<u64>,
    pub eta_ms: Option<u64>,
}

impl ProgressEvent {
    pub fn to_job_update(&self) -> JobUpdate {
        JobUpdate {
            id: self.job_id,
            state: self.state.clone(),
            bytes_uploaded: self.bytes_uploaded,
            eta_ms: self.eta_ms,
        }
    }
}

/// Manages progress updates and broadcasts them to UI
pub struct ProgressBroadcaster {
    /// Channel for progress events
    tx: Arc<mpsc::UnboundedSender<ProgressEvent>>,
    /// Recent state snapshots for each job
    states: Arc<DashMap<JobId, JobState>>,
}

impl ProgressBroadcaster {
    pub fn new(tx: mpsc::UnboundedSender<ProgressEvent>) -> Self {
        ProgressBroadcaster {
            tx: Arc::new(tx),
            states: Arc::new(DashMap::new()),
        }
    }

    /// Report progress update
    pub fn report(&self, event: ProgressEvent) {
        // Store state snapshot
        self.states.insert(event.job_id, event.state.clone());

        // Send to listener (UI layer will collect and emit queue:tick)
        let _ = self.tx.send(event);
    }

    /// Get current state of a job
    pub fn get_state(&self, job_id: JobId) -> Option<JobState> {
        self.states.get(&job_id).map(|v| v.clone())
    }

    /// Get all job states
    pub fn get_all_states(&self) -> Vec<(JobId, JobState)> {
        self.states
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Clean up state for completed job
    pub fn cleanup(&self, job_id: JobId) {
        self.states.remove(&job_id);
    }
}

impl Clone for ProgressBroadcaster {
    fn clone(&self) -> Self {
        ProgressBroadcaster {
            tx: Arc::clone(&self.tx),
            states: Arc::clone(&self.states),
        }
    }
}

/// Collects progress events and batches them into QueueTick events
pub struct TickCollector {
    rx: mpsc::UnboundedReceiver<ProgressEvent>,
    batch_size: usize,
    batch_timeout_ms: u64,
}

impl TickCollector {
    pub fn new(
        rx: mpsc::UnboundedReceiver<ProgressEvent>,
        batch_size: usize,
        batch_timeout_ms: u64,
    ) -> Self {
        TickCollector {
            rx,
            batch_size,
            batch_timeout_ms,
        }
    }

    /// Collect events into QueueTick
    pub async fn next_tick(&mut self) -> Option<QueueTick> {
        let mut updates = Vec::new();
        let start_time = std::time::Instant::now();

        loop {
            let timeout = std::time::Duration::from_millis(self.batch_timeout_ms);
            let remaining = timeout.saturating_sub(start_time.elapsed());

            let event = if remaining.is_zero() {
                None
            } else {
                tokio::time::timeout(remaining, self.rx.recv())
                    .await
                    .ok()
                    .flatten()
            };

            match event {
                Some(evt) => {
                    updates.push(evt.to_job_update());
                    if updates.len() >= self.batch_size {
                        break;
                    }
                }
                None => {
                    if !updates.is_empty() {
                        break;
                    }
                    return None; // Channel closed
                }
            }
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Some(QueueTick { updates, ts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_broadcaster_report() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let broadcaster = ProgressBroadcaster::new(tx);
        let job_id = JobId::new();

        let event = ProgressEvent {
            job_id,
            state: JobState::Queued,
            bytes_uploaded: None,
            eta_ms: None,
        };

        broadcaster.report(event);

        let state = broadcaster.get_state(job_id);
        assert!(matches!(state, Some(JobState::Queued)));
    }

    #[test]
    fn test_progress_broadcaster_cleanup() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let broadcaster = ProgressBroadcaster::new(tx);
        let job_id = JobId::new();

        let event = ProgressEvent {
            job_id,
            state: JobState::Queued,
            bytes_uploaded: None,
            eta_ms: None,
        };

        broadcaster.report(event);
        assert!(broadcaster.get_state(job_id).is_some());

        broadcaster.cleanup(job_id);
        assert!(broadcaster.get_state(job_id).is_none());
    }
}
