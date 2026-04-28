// ============================================================================
// Retry Logic with Exponential Backoff
// ============================================================================

use crate::types::AppErrorView;
use rand::Rng;
use std::time::Duration;

/// Classification of errors for retry strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Temporary errors that should be retried (rate limit, network timeout)
    Retryable,
    /// Permanent errors that should not be retried (invalid file, auth)
    Permanent,
}

/// Classifies errors and determines retry strategy
pub fn classify_error(error: &AppErrorView) -> ErrorClass {
    match error.code.as_str() {
        // Retryable: temporary network/service issues
        "RATE_LIMIT" | "NETWORK_ERROR" | "TIMEOUT" => ErrorClass::Retryable,
        // Permanent: input/auth errors
        "INVALID_FILE" | "AUTH_FAILED" | "INVALID_INPUT" => ErrorClass::Permanent,
        // Default: treat as permanent to avoid infinite loops
        _ => ErrorClass::Permanent,
    }
}

/// Exponential backoff calculator with jitter
pub struct BackoffCalculator {
    /// Initial delay in milliseconds
    initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    max_delay_ms: u64,
    /// Exponential base (typically 2)
    base: f64,
}

impl BackoffCalculator {
    pub fn new(initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        BackoffCalculator {
            initial_delay_ms,
            max_delay_ms,
            base: 2.0,
        }
    }

    /// Calculate delay for attempt number (0-indexed)
    /// Formula: min(initial_delay * base^attempt, max_delay) + jitter
    pub fn calculate(&self, attempt: u32) -> Duration {
        let base_delay = (self.initial_delay_ms as f64 * self.base.powi(attempt as i32)) as u64;
        let clamped = base_delay.min(self.max_delay_ms);

        // Add jitter: ±10% of calculated delay
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.9..1.1);
        let with_jitter = (clamped as f64 * jitter_factor) as u64;

        Duration::from_millis(with_jitter)
    }
}

impl Default for BackoffCalculator {
    fn default() -> Self {
        // Start at 100ms, max out at 30 seconds
        BackoffCalculator::new(100, 30_000)
    }
}

/// Retry policy for jobs
pub struct RetryPolicy {
    /// Maximum attempts (including initial)
    max_attempts: u32,
    /// Backoff calculator
    backoff: BackoffCalculator,
    /// Only retry if error is classified as retryable
    classify: fn(&AppErrorView) -> ErrorClass,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32) -> Self {
        RetryPolicy {
            max_attempts,
            backoff: BackoffCalculator::default(),
            classify: classify_error,
        }
    }

    /// Check if we should retry (not out of attempts and error is retryable)
    pub fn should_retry(&self, current_attempt: u32, error: &AppErrorView) -> bool {
        if current_attempt >= self.max_attempts {
            return false;
        }
        (self.classify)(error) == ErrorClass::Retryable
    }

    /// Get delay before next retry
    pub fn delay_before_retry(&self, current_attempt: u32) -> Duration {
        self.backoff.calculate(current_attempt)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy::new(3) // 3 attempts (initial + 2 retries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification_retryable() {
        let error = AppErrorView::rate_limit(Some(60));
        assert_eq!(classify_error(&error), ErrorClass::Retryable);
    }

    #[test]
    fn test_error_classification_permanent() {
        let error = AppErrorView::invalid_file("test");
        assert_eq!(classify_error(&error), ErrorClass::Permanent);
    }

    #[test]
    fn test_backoff_calculator() {
        let calc = BackoffCalculator::new(100, 30_000);

        // Attempt 0: ~100ms
        let delay0 = calc.calculate(0);
        assert!(delay0.as_millis() >= 90 && delay0.as_millis() <= 110);

        // Attempt 1: ~200ms
        let delay1 = calc.calculate(1);
        assert!(delay1.as_millis() >= 180 && delay1.as_millis() <= 220);

        // Attempt 2: ~400ms
        let delay2 = calc.calculate(2);
        assert!(delay2.as_millis() >= 360 && delay2.as_millis() <= 440);

        // Capped at max (plus jitter)
        let delay10 = calc.calculate(10);
        // Max is 30_000 plus up to 10% jitter
        assert!(delay10.as_millis() <= 33_000);
    }

    #[test]
    fn test_retry_policy_should_retry() {
        let policy = RetryPolicy::default();

        let retryable_error = AppErrorView::rate_limit(Some(60));
        assert!(policy.should_retry(0, &retryable_error));
        assert!(policy.should_retry(1, &retryable_error));
        assert!(policy.should_retry(2, &retryable_error));
        assert!(!policy.should_retry(3, &retryable_error)); // Out of attempts (3 max)

        let permanent_error = AppErrorView::invalid_file("test");
        assert!(!policy.should_retry(0, &permanent_error)); // Permanent error
    }

    #[test]
    fn test_retry_policy_delay() {
        let policy = RetryPolicy::default();

        let delay0 = policy.delay_before_retry(0);
        let delay1 = policy.delay_before_retry(1);

        // delay1 should be roughly 2x delay0
        assert!(delay1.as_millis() > delay0.as_millis());
    }
}
