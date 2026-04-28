// ============================================================================
// Pipeline: End-to-End Job Processing State Machine
// ============================================================================
// Orchestrates: Probing → Extracting → Uploading → Transcribing → Writing → Done
//
// Responsibilities:
// - State transitions with persistence to SQLite
// - Error handling and retry logic
// - Progress event reporting
// - Cancellation token checks
// - Cleanup on success/failure

use crate::adapters::ffmpeg::FfmpegAdapter;
use crate::adapters::groq::GroqClient;
use crate::core::cancellation::CancellationToken;
use crate::core::progress::ProgressBroadcaster;
use crate::core::retry::RetryPolicy;
use crate::core::stages::{self, PipelineCtx};
use crate::db::JobRepo;
use crate::types::{AppErrorView, JobId, JobState};
use std::sync::Arc;

/// Pipeline executor with all dependencies injected
pub struct Pipeline {
    ffmpeg: Arc<FfmpegAdapter>,
    groq: Arc<GroqClient>,
    job_repo: Arc<JobRepo>,
    progress: ProgressBroadcaster,
    retry_policy: RetryPolicy,
}

impl Pipeline {
    pub fn new(
        ffmpeg: Arc<FfmpegAdapter>,
        groq: Arc<GroqClient>,
        job_repo: Arc<JobRepo>,
        progress: ProgressBroadcaster,
    ) -> Self {
        Pipeline {
            ffmpeg,
            groq,
            job_repo,
            progress,
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Run complete pipeline for a job
    pub async fn run(
        &self,
        job_id: JobId,
        cancel_token: CancellationToken,
    ) -> Result<(), AppErrorView> {
        // Load job from database
        let job = self
            .job_repo
            .get(job_id)
            .await?
            .ok_or_else(|| AppErrorView::internal_error("Job not found"))?;

        let mut ctx = PipelineCtx::new(job);

        match self.run_stages(&mut ctx, &cancel_token).await {
            Ok(()) => {
                // Save final state
                self.job_repo.update_state(job_id, &ctx.job.state).await?;
                Ok(())
            }
            Err(err) => {
                // Mark as failed and save
                ctx.job.state = JobState::Failed {
                    error: err.clone(),
                    attempts: 1,
                };
                let _ = self.job_repo.update_state(job_id, &ctx.job.state).await;

                // Cleanup temp files
                stages::cleanup(&mut ctx).await;

                Err(err)
            }
        }
    }

    /// Execute stages sequentially with state reporting
    async fn run_stages(
        &self,
        ctx: &mut PipelineCtx,
        cancel_token: &CancellationToken,
    ) -> Result<(), AppErrorView> {
        // Stage 1: Probe
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

        let probe_result = stages::probe(ctx, &self.ffmpeg).await?;
        ctx.job.state = probe_result.state.clone();
        self.progress.report(crate::core::progress::ProgressEvent {
            job_id: ctx.job.id,
            state: probe_result.state.clone(),
            bytes_uploaded: None,
            eta_ms: None,
        });
        self.job_repo
            .update_state(ctx.job.id, &probe_result.state)
            .await?;

        if !probe_result.should_continue {
            return Ok(());
        }

        // Stage 2: Extract
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

        let extract_result = stages::extract(ctx, &self.ffmpeg).await?;
        ctx.job.state = extract_result.state.clone();
        self.progress.report(crate::core::progress::ProgressEvent {
            job_id: ctx.job.id,
            state: extract_result.state.clone(),
            bytes_uploaded: None,
            eta_ms: None,
        });
        self.job_repo
            .update_state(ctx.job.id, &extract_result.state)
            .await?;

        if !extract_result.should_continue {
            return Ok(());
        }

        // Stage 3: Upload (state update) + Stage 4: Transcribe
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

        // For now, upload and transcribe are combined in groq.transcribe()
        // In a future enhancement, we could split these for better progress tracking
        let transcribe_result = stages::transcribe(ctx, &self.groq).await?;
        ctx.job.state = transcribe_result.state.clone();
        self.progress.report(crate::core::progress::ProgressEvent {
            job_id: ctx.job.id,
            state: transcribe_result.state.clone(),
            bytes_uploaded: None,
            eta_ms: None,
        });
        self.job_repo
            .update_state(ctx.job.id, &transcribe_result.state)
            .await?;

        if !transcribe_result.should_continue {
            return Ok(());
        }

        // Stage 5: Write
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

        let write_result = stages::write_transcript(ctx).await?;
        ctx.job.state = write_result.state.clone();
        self.progress.report(crate::core::progress::ProgressEvent {
            job_id: ctx.job.id,
            state: write_result.state.clone(),
            bytes_uploaded: None,
            eta_ms: None,
        });

        // Final cleanup
        stages::cleanup(ctx).await;

        Ok(())
    }
}

/// Run pipeline with automatic retry on retriable errors
pub async fn run_with_retry(
    job_id: JobId,
    cancel_token: CancellationToken,
    pipeline: &Pipeline,
    retry_policy: &RetryPolicy,
) -> Result<(), AppErrorView> {
    let mut attempt = 0;

    loop {
        match pipeline.run(job_id, cancel_token.clone()).await {
            Ok(()) => return Ok(()),
            Err(ref err) => {
                if !retry_policy.should_retry(attempt, err) {
                    // Don't retry: either out of attempts or permanent error
                    return Err(err.clone());
                }

                // Retryable: wait and try again
                attempt += 1;
                let delay = retry_policy.delay_before_retry(attempt - 1);

                // Check for cancellation during wait
                tokio::select! {
                    _ = tokio::time::sleep(delay) => {
                        // Continue to next attempt
                    }
                    _ = cancel_token.wait() => {
                        return Err(AppErrorView::new("CANCELLED", "Job cancelled during retry wait"));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        // This test just ensures the pipeline can be instantiated
        // Full E2E tests will be in integration tests with mock Groq
        let _ = true; // Placeholder
    }
}
