// ============================================================================
// Pipeline: End-to-End Job Processing State Machine
// ============================================================================
// Orchestrates: Probing → Extracting → Chunking → Transcribing → Stitching → Writing → Done
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
use crate::types::{AppErrorView, JobId, JobState, TranscriptSegment};
use std::sync::Arc;

/// Pipeline executor with all dependencies injected
pub struct Pipeline {
    ffmpeg: Arc<FfmpegAdapter>,
    groq: Arc<GroqClient>,
    job_repo: Arc<JobRepo>,
    progress: ProgressBroadcaster,
    #[allow(dead_code)]
    retry_policy: RetryPolicy,
    postprocess_model: String, // Model for Llama postprocessing (e.g. llama-3.1-8b-instant)
}

impl Pipeline {
    pub fn new(
        ffmpeg: Arc<FfmpegAdapter>,
        groq: Arc<GroqClient>,
        job_repo: Arc<JobRepo>,
        progress: ProgressBroadcaster,
        postprocess_model: String,
    ) -> Self {
        Pipeline {
            ffmpeg,
            groq,
            job_repo,
            progress,
            retry_policy: RetryPolicy::default(),
            postprocess_model,
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

        // Stage 3: Chunking (if needed for files >100 MB)
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

        let chunk_result = stages::chunk(ctx, &self.ffmpeg).await?;
        ctx.job.state = chunk_result.state.clone();
        self.progress.report(crate::core::progress::ProgressEvent {
            job_id: ctx.job.id,
            state: chunk_result.state.clone(),
            bytes_uploaded: None,
            eta_ms: None,
        });
        self.job_repo
            .update_state(ctx.job.id, &chunk_result.state)
            .await?;

        if !chunk_result.should_continue {
            return Ok(());
        }

        // Stage 4: Transcribe (handles single chunk or multiple chunks)
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

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

        // Stage 5: Stitching (if chunking was used)
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

        if ctx.chunks.len() > 1 {
            // Multiple chunks: need to stitch
            let stitch_result = stages::stitch_transcript(ctx).await?;
            ctx.job.state = stitch_result.state.clone();
            self.progress.report(crate::core::progress::ProgressEvent {
                job_id: ctx.job.id,
                state: stitch_result.state.clone(),
                bytes_uploaded: None,
                eta_ms: None,
            });
            self.job_repo
                .update_state(ctx.job.id, &stitch_result.state)
                .await?;

            if !stitch_result.should_continue {
                return Ok(());
            }
        } else {
            // Single chunk: just copy segments and text
            if !ctx.chunk_transcripts.is_empty() {
                let chunk_tx = &ctx.chunk_transcripts[0];
                ctx.transcript_raw = chunk_tx.text.clone();
                ctx.segments = chunk_tx
                    .segments
                    .iter()
                    .map(|seg| TranscriptSegment {
                        start_ms: (seg.start * 1000.0) as u64 + chunk_tx.chunk_start_ms,
                        end_ms: (seg.end * 1000.0) as u64 + chunk_tx.chunk_start_ms,
                        text: seg.text.clone(),
                    })
                    .collect();
            }
        }

        // Stage 6: Postprocessing (optional)
        if cancel_token.is_cancelled() {
            return Err(AppErrorView::new("CANCELLED", "Job cancelled by user"));
        }

        let postprocess_result =
            stages::postprocess_transcript(ctx, &self.groq, &self.postprocess_model).await?;
        ctx.job.state = postprocess_result.state.clone();
        self.progress.report(crate::core::progress::ProgressEvent {
            job_id: ctx.job.id,
            state: postprocess_result.state.clone(),
            bytes_uploaded: None,
            eta_ms: None,
        });
        self.job_repo
            .update_state(ctx.job.id, &postprocess_result.state)
            .await?;

        if !postprocess_result.should_continue {
            return Ok(());
        }

        // Stage 7: Write
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

    #[test]
    fn test_pipeline_creation() {
        // This test just ensures the pipeline can be instantiated
        // Full E2E tests will be in integration tests with mock Groq
        let _ = true; // Placeholder
    }
}
