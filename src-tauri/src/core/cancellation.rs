// ============================================================================
// Cancellation & Pause/Resume
// ============================================================================

use crate::types::JobId;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Notify;

/// Cancellation token for a job
#[derive(Clone)]
pub struct CancellationToken {
    is_cancelled: Arc<std::sync::atomic::AtomicBool>,
    notify: Arc<Notify>,
}

impl CancellationToken {
    pub fn new() -> Self {
        CancellationToken {
            is_cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Request cancellation
    pub fn cancel(&self) {
        self.is_cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    /// Wait for cancellation signal
    pub async fn wait(&self) {
        self.notify.notified().await;
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for cancellation tokens of all jobs
pub struct CancellationManager {
    tokens: Arc<DashMap<JobId, CancellationToken>>,
}

impl CancellationManager {
    pub fn new() -> Self {
        CancellationManager {
            tokens: Arc::new(DashMap::new()),
        }
    }

    /// Get or create cancellation token for a job
    pub fn get_or_create(&self, job_id: JobId) -> CancellationToken {
        self.tokens.entry(job_id).or_default().clone()
    }

    /// Cancel a specific job
    pub fn cancel(&self, job_id: JobId) {
        if let Some(token) = self.tokens.get(&job_id) {
            token.cancel();
        }
    }

    /// Cancel all jobs
    pub fn cancel_all(&self) {
        for entry in self.tokens.iter() {
            entry.value().cancel();
        }
    }

    /// Remove token (cleanup after job completion)
    pub fn remove(&self, job_id: JobId) {
        self.tokens.remove(&job_id);
    }
}

impl Default for CancellationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellation_token_creation() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_cancel() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_wait() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            token_clone.cancel();
        });

        token.wait().await;
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_manager() {
        let manager = CancellationManager::new();
        let job_id = JobId::new();

        let token1 = manager.get_or_create(job_id);
        let token2 = manager.get_or_create(job_id);

        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        token1.cancel();

        assert!(token2.is_cancelled());
    }
}
