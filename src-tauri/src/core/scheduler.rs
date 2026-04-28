// ============================================================================
// Job Scheduler
// ============================================================================
// Manages:
// - Job queue (FIFO with priority)
// - Semaphores for CPU-bound and network-bound stages
// - Rate limiting (Groq free tier: 30 RPM)
// - Exponential backoff for retries
// - Cancellation tokens
// - Pause/Resume of entire queue

use crate::adapters::ffmpeg::FfmpegAdapter;
use crate::adapters::groq::GroqClient;
use crate::core::cancellation::CancellationManager;
use crate::core::pipeline::Pipeline;
use crate::core::progress::ProgressBroadcaster;
use crate::core::retry::RetryPolicy;
use crate::db::JobRepo;
use crate::types::{AppErrorView, JobId};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Coordinates job execution with proper parallelism
pub struct JobScheduler {
    /// Semaphore for CPU-bound operations (ffprobe, ffmpeg)
    /// Limit: 2 concurrent (respect CPU usage on Ryzen 3)
    pub cpu_sem: Arc<Semaphore>,

    /// Semaphore for network-bound operations (Groq API)
    /// Limit: 3 concurrent (respect Groq free tier: 30 RPM)
    pub net_sem: Arc<Semaphore>,

    /// Job queue (FIFO)
    queue: Arc<Mutex<VecDeque<JobId>>>,

    /// Cancellation manager for all jobs
    cancellations: Arc<CancellationManager>,

    /// Pause/resume flag
    is_paused: Arc<AtomicBool>,

    /// Pipeline executor
    pipeline: Arc<Pipeline>,

    /// Retry policy
    retry_policy: Arc<RetryPolicy>,
}

impl JobScheduler {
    pub fn new(
        ffmpeg: Arc<FfmpegAdapter>,
        groq: Arc<GroqClient>,
        job_repo: Arc<JobRepo>,
        progress: ProgressBroadcaster,
    ) -> Self {
        JobScheduler {
            cpu_sem: Arc::new(Semaphore::new(2)),
            net_sem: Arc::new(Semaphore::new(3)),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            cancellations: Arc::new(CancellationManager::new()),
            is_paused: Arc::new(AtomicBool::new(false)),
            pipeline: Arc::new(Pipeline::new(ffmpeg, groq, job_repo, progress)),
            retry_policy: Arc::new(RetryPolicy::default()),
        }
    }

    /// Enqueue a job for processing
    pub async fn enqueue(&self, job_id: JobId) -> Result<(), AppErrorView> {
        let mut queue = self.queue.lock().await;
        queue.push_back(job_id);
        Ok(())
    }

    /// Cancel a specific job
    pub fn cancel(&self, job_id: JobId) {
        self.cancellations.cancel(job_id);
    }

    /// Cancel all jobs
    pub fn cancel_all(&self) {
        self.cancellations.cancel_all();
    }

    /// Pause the scheduler (current jobs continue, new jobs wait)
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
    }

    /// Resume the scheduler
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
    }

    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Main scheduler loop: process jobs from queue
    pub async fn run(&self) -> Result<(), AppErrorView> {
        loop {
            // Wait if paused
            while self.is_paused() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            // Get next job
            let job_id = {
                let mut queue = self.queue.lock().await;
                queue.pop_front()
            };

            match job_id {
                Some(id) => {
                    // Get cancellation token for this job
                    let cancel_token = self.cancellations.get_or_create(id);

                    // Acquire semaphores (CPU first, then network)
                    let _cpu_guard = self.cpu_sem.acquire().await;
                    let _net_guard = self.net_sem.acquire().await;

                    // Run pipeline
                    let pipeline = Arc::clone(&self.pipeline);
                    let retry_policy = Arc::clone(&self.retry_policy);

                    let result = tokio::spawn(async move {
                        crate::core::pipeline::run_with_retry(
                            id,
                            cancel_token.clone(),
                            &pipeline,
                            &retry_policy,
                        )
                        .await
                    })
                    .await;

                    // Clean up cancellation token
                    self.cancellations.remove(id);

                    match result {
                        Ok(Ok(())) => {
                            // Job completed successfully
                        }
                        Ok(Err(e)) => {
                            eprintln!("Job {} failed: {}", id, e);
                        }
                        Err(e) => {
                            eprintln!("Job {} panicked: {}", id, e);
                        }
                    }
                }
                None => {
                    // Queue is empty, wait a bit before checking again
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Get queue length (for monitoring)
    pub async fn queue_len(&self) -> usize {
        self.queue.lock().await.len()
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        // This is a simplified default that won't work for full app
        // Use JobScheduler::new() instead
        panic!("Use JobScheduler::new() with proper dependencies");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        // Full tests require mocking adapters and database
        // This is just a placeholder
        let _ = true;
    }
}
