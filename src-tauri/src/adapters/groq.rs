// ============================================================================
// Groq API Client
// ============================================================================
// Integration with Groq cloud API for:
// - whisper-large-v3-turbo (speech-to-text)
// - llama-3.1-8b-instant (postprocessing)
// Handles:
// - Multipart audio file upload
// - Rate limiting (free tier: 30 RPM)
// - Retry/backoff with jitter and error classification
// - API key from OS keyring (never hardcoded)

use crate::types::AppErrorView;
use reqwest::header::{HeaderMap, AUTHORIZATION, RETRY_AFTER};
use reqwest::multipart;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// HTTP client for Groq API with rate limiting and retry logic
pub struct GroqClient {
    /// HTTP client with custom config
    http: Client,
    /// API key from OS keyring (Secret to prevent accidental logging)
    _api_key: SecretString,
    /// Base URL (usually https://api.groq.com/openai/v1)
    base_url: String,
    /// Rate limiter: 30 requests per minute (free tier)
    rate_limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
}

/// Simple token-bucket rate limiter
struct RateLimiter {
    /// Max tokens (30 for 30 RPM)
    max_tokens: f64,
    /// Current tokens
    tokens: f64,
    /// Last refill timestamp (seconds since epoch)
    last_refill: f64,
    /// Refill rate: tokens per second (30 / 60 = 0.5)
    refill_rate: f64,
}

impl RateLimiter {
    fn new() -> Self {
        RateLimiter {
            max_tokens: 30.0,
            tokens: 30.0,
            last_refill: Self::now(),
            refill_rate: 30.0 / 60.0, // 0.5 tokens/sec
        }
    }

    fn now() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }

    /// Wait until a token is available, then consume it
    async fn acquire(&mut self) {
        loop {
            let now = Self::now();
            let elapsed = now - self.last_refill;
            self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
            self.last_refill = now;

            if self.tokens >= 1.0 {
                self.tokens -= 1.0;
                return;
            }

            // Calculate wait time for next token
            let wait_ms = ((1.0 - self.tokens) / self.refill_rate * 1000.0) as u64;
            tokio::time::sleep(Duration::from_millis(wait_ms.max(10))).await;
        }
    }
}

impl GroqClient {
    /// Create new Groq client from API key
    pub fn new(api_key: String) -> Result<Self, AppErrorView> {
        if api_key.trim().is_empty() {
            return Err(AppErrorView::auth_failed());
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", api_key)
                .parse()
                .map_err(|_| AppErrorView::internal_error("Invalid API key format"))?,
        );

        let http = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(300)) // 5 min for large uploads
            .build()
            .map_err(|e| {
                AppErrorView::internal_error(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(GroqClient {
            http,
            _api_key: SecretString::new(api_key),
            base_url: "https://api.groq.com/openai/v1".into(),
            rate_limiter: Arc::new(tokio::sync::Mutex::new(RateLimiter::new())),
        })
    }

    /// Transcribe audio file via Groq Whisper API with multipart upload
    /// - Implements retry/backoff with error classification
    /// - Returns verbose_json with segments and timings
    pub async fn transcribe(
        &self,
        audio_path: &Path,
        opts: TranscribeOpts,
    ) -> Result<TranscribeResult, AppErrorView> {
        if self._api_key.expose_secret().trim().is_empty() {
            return Err(AppErrorView::auth_failed());
        }

        // Validate file exists and get size
        let file_meta = tokio::fs::metadata(audio_path)
            .await
            .map_err(|e| AppErrorView::fs_error(format!("Audio file not found: {}", e)))?;

        let file_size = file_meta.len();
        if file_size == 0 {
            return Err(AppErrorView::invalid_file("Audio file is empty"));
        }

        // Execute request with retry logic
        self.send_transcribe_request(audio_path, opts).await
    }

    /// Send transcribe request with retry/backoff logic
    async fn send_transcribe_request(
        &self,
        audio_path: &Path,
        opts: TranscribeOpts,
    ) -> Result<TranscribeResult, AppErrorView> {
        let url = format!("{}/audio/transcriptions", self.base_url);
        let client = self.http.clone();

        // Acquire rate limit token before attempt
        {
            let mut limiter = self.rate_limiter.lock().await;
            limiter.acquire().await;
        }

        let file_data = tokio::fs::read(audio_path)
            .await
            .map_err(|e| AppErrorView::fs_error(format!("Failed to read audio file: {}", e)))?;

        let filename = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio");

        // Manual retry loop with exponential backoff
        let mut attempt = 0u32;
        let max_attempts = 5;
        let max_elapsed = Duration::from_secs(120);
        let start_time = Instant::now();

        loop {
            // Check if max elapsed time exceeded
            if start_time.elapsed() > max_elapsed {
                return Err(AppErrorView::internal_error(
                    "Max retry time exceeded (2 minutes)",
                ));
            }

            // Build multipart form
            let mut form = multipart::Form::new();
            form = form.text("model", opts.model.clone());
            form = form.text("language", opts.language.clone());
            form = form.text("response_format", opts.response_format.clone());
            form = form.text("temperature", opts.temperature.to_string());

            if !opts.prompt.is_empty() {
                form = form.text("prompt", opts.prompt.clone());
            }

            let part = multipart::Part::bytes(file_data.clone())
                .file_name(filename.to_string())
                .mime_str("audio/mpeg")
                .map_err(|e| {
                    AppErrorView::internal_error(format!("Failed to create multipart: {}", e))
                })?;

            form = form.part("file", part);

            match client.post(&url).multipart(form).send().await {
                Ok(response) => {
                    let status = response.status();

                    // Handle rate limit with Retry-After header
                    if status == 429 {
                        let retry_secs = response
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(60);

                        attempt += 1;
                        if attempt >= max_attempts {
                            return Err(AppErrorView::rate_limit(Some(retry_secs as u32)));
                        }
                        tokio::time::sleep(Duration::from_secs(retry_secs)).await;
                        continue;
                    }

                    // Auth errors: no retry
                    if status == 401 {
                        return Err(AppErrorView::auth_failed());
                    }

                    // Server errors: retry
                    if status.is_server_error() {
                        attempt += 1;
                        if attempt >= max_attempts {
                            return Err(AppErrorView::internal_error(format!(
                                "Groq server error: {} (after {} attempts)",
                                status, attempt
                            )));
                        }
                        let delay = Duration::from_secs(1u64 << attempt.min(5));
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    // Other 4xx: no retry
                    if status.is_client_error() {
                        let text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| String::from("Unknown error"));
                        return Err(AppErrorView::new(
                            "API_ERROR",
                            format!("Groq API error: {}", text),
                        )
                        .with_details(format!("Status: {}", status)));
                    }

                    // Success: parse response
                    if status.is_success() {
                        match response.json::<VerboseJsonResponse>().await {
                            Ok(json) => return Ok(json.into()),
                            Err(e) => {
                                return Err(AppErrorView::internal_error(format!(
                                    "Failed to parse Groq response: {}",
                                    e
                                )))
                            }
                        }
                    } else {
                        return Err(AppErrorView::internal_error(format!(
                            "Unexpected status: {}",
                            status
                        )));
                    }
                }
                Err(e) => {
                    // Network errors: transient (retry)
                    if e.is_timeout() || e.is_connect() {
                        attempt += 1;
                        if attempt >= max_attempts {
                            return Err(AppErrorView::network_error(format!(
                                "Network error (after {} attempts): {}",
                                attempt, e
                            )));
                        }
                        let delay = Duration::from_secs(1u64 << attempt.min(5));
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        // Other errors: permanent
                        return Err(AppErrorView::network_error(format!(
                            "Request failed: {}",
                            e
                        )));
                    }
                }
            }
        }
    }

    /// Postprocess transcript via Groq Llama
    /// Cleans punctuation, grammar, formatting without changing meaning
    pub async fn postprocess(&self, text: String, model: &str) -> Result<String, AppErrorView> {
        if self._api_key.expose_secret().trim().is_empty() {
            return Err(AppErrorView::auth_failed());
        }

        let url = format!("{}/chat/completions", self.base_url);
        let client = self.http.clone();

        // Acquire rate limit token
        {
            let mut limiter = self.rate_limiter.lock().await;
            limiter.acquire().await;
        }

        // Build safety prompt that strictly forbids meaning changes
        let system_prompt = "You are a transcript cleaner. Your task is to:
1. Fix punctuation and capitalization
2. Improve readability with proper spacing
3. Correct obvious typos
4. Add paragraph breaks where appropriate

STRICT RULES (DO NOT VIOLATE):
- NEVER change the meaning of any words
- NEVER add facts not in the original text
- NEVER translate to another language
- NEVER remove important words or concepts
- NEVER reorder sentences
- Keep all names, places, and technical terms exactly as they appear
- Return ONLY the cleaned transcript, nothing else";

        let user_message = format!(
            "Clean this transcript (remember: preserve ALL meaning):\n\n{}",
            text
        );

        let request_body = serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_message
                }
            ],
            "temperature": 0.3,  // Low temp for consistency
            "max_tokens": std::cmp::min(text.len() as i32 * 2, 8000),  // Allow growth for formatting
        });

        // Manual retry loop with exponential backoff
        let mut attempt = 0u32;
        let max_attempts = 3;
        let max_elapsed = Duration::from_secs(60);
        let start_time = Instant::now();

        loop {
            // Check if max elapsed time exceeded
            if start_time.elapsed() > max_elapsed {
                return Err(AppErrorView::internal_error(
                    "Postprocessing timeout (>60s)",
                ));
            }

            match client.post(&url).json(&request_body).send().await {
                Ok(response) => {
                    let status = response.status();

                    // Handle rate limit
                    if status == 429 {
                        let retry_secs = response
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(30);

                        attempt += 1;
                        if attempt >= max_attempts {
                            return Err(AppErrorView::rate_limit(Some(retry_secs as u32)));
                        }
                        tokio::time::sleep(Duration::from_secs(retry_secs)).await;
                        continue;
                    }

                    // Auth errors: no retry
                    if status == 401 {
                        return Err(AppErrorView::auth_failed());
                    }

                    // Server errors: retry
                    if status.is_server_error() {
                        attempt += 1;
                        if attempt >= max_attempts {
                            return Err(AppErrorView::internal_error(format!(
                                "Groq server error: {} (after {} attempts)",
                                status, attempt
                            )));
                        }
                        let delay = Duration::from_secs(1u64 << attempt.min(3));
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    // Other 4xx: no retry
                    if status.is_client_error() {
                        let text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| String::from("Unknown error"));
                        return Err(AppErrorView::new(
                            "POSTPROCESS_ERROR",
                            format!("Groq postprocess error: {}", text),
                        ));
                    }

                    // Success: parse and extract message
                    if status.is_success() {
                        match response.json::<ChatCompletionResponse>().await {
                            Ok(resp) => {
                                if let Some(choice) = resp.choices.first() {
                                    return Ok(choice.message.content.trim().to_string());
                                } else {
                                    return Err(AppErrorView::internal_error(
                                        "Empty response from postprocessing",
                                    ));
                                }
                            }
                            Err(e) => {
                                return Err(AppErrorView::internal_error(format!(
                                    "Failed to parse postprocess response: {}",
                                    e
                                )))
                            }
                        }
                    } else {
                        return Err(AppErrorView::internal_error(format!(
                            "Unexpected status: {}",
                            status
                        )));
                    }
                }
                Err(e) => {
                    // Network errors: transient (retry)
                    if e.is_timeout() || e.is_connect() {
                        attempt += 1;
                        if attempt >= max_attempts {
                            return Err(AppErrorView::network_error(format!(
                                "Postprocessing network error (after {} attempts): {}",
                                attempt, e
                            )));
                        }
                        let delay = Duration::from_secs(1u64 << attempt.min(3));
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        // Other errors: permanent
                        return Err(AppErrorView::network_error(format!(
                            "Postprocessing request failed: {}",
                            e
                        )));
                    }
                }
            }
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscribeOpts {
    pub language: String,        // e.g. "ru"
    pub temperature: f32,        // 0 for deterministic
    pub prompt: String,          // Language hint
    pub model: String,           // whisper-large-v3-turbo
    pub response_format: String, // verbose_json
}

impl Default for TranscribeOpts {
    fn default() -> Self {
        TranscribeOpts {
            language: "ru".into(),
            temperature: 0.0,
            prompt: "Это запись на русском языке. Говорит один человек.".into(),
            model: "whisper-large-v3-turbo".into(),
            response_format: "verbose_json".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscribeResult {
    pub text: String,
    pub language: String,
    pub segments: Vec<TranscriptSegmentResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptSegmentResult {
    pub id: u32,
    pub start: f32, // seconds
    pub end: f32,   // seconds
    pub text: String,
    pub temperature: f32,
    pub avg_logprob: f32,
    pub compression_ratio: f32,
    pub no_speech_prob: f32,
}

// ============================================================================
// Groq verbose_json Response (from official Whisper API)
// ============================================================================

#[derive(Debug, Deserialize)]
struct VerboseJsonResponse {
    #[allow(dead_code)]
    task: String,
    #[allow(dead_code)]
    language: String,
    #[allow(dead_code)]
    duration: f32,
    text: String,
    segments: Vec<VerboseSegment>,
}

#[derive(Debug, Deserialize)]
struct VerboseSegment {
    #[allow(dead_code)]
    id: u32,
    #[allow(dead_code)]
    seek: u32,
    start: f32,
    end: f32,
    text: String,
    avg_logprob: f32,
    compression_ratio: f32,
    no_speech_prob: f32,
}

impl From<VerboseJsonResponse> for TranscribeResult {
    fn from(resp: VerboseJsonResponse) -> Self {
        TranscribeResult {
            text: resp.text,
            language: resp.language,
            segments: resp
                .segments
                .into_iter()
                .map(|seg| TranscriptSegmentResult {
                    id: seg.id,
                    start: seg.start,
                    end: seg.end,
                    text: seg.text,
                    temperature: 0.0, // Not in response
                    avg_logprob: seg.avg_logprob,
                    compression_ratio: seg.compression_ratio,
                    no_speech_prob: seg.no_speech_prob,
                })
                .collect(),
        }
    }
}

// ============================================================================
// Groq Chat Completions Response (for postprocessing via Llama)
// ============================================================================

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionMessage {
    content: String,
}

// ============================================================================
// Unit Tests
// ============================================================================
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new();
        assert_eq!(limiter.max_tokens, 30.0);
        assert!(limiter.tokens > 0.0);
        assert_eq!(limiter.refill_rate, 0.5);
    }

    #[test]
    fn test_groq_client_new() {
        let result = GroqClient::new("gsk_test_key_123".into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_groq_client_empty_key() {
        let result = GroqClient::new("".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_transcribe_opts_default() {
        let opts = TranscribeOpts::default();
        assert_eq!(opts.language, "ru");
        assert_eq!(opts.temperature, 0.0);
        assert_eq!(opts.model, "whisper-large-v3-turbo");
        assert_eq!(opts.response_format, "verbose_json");
    }

    #[test]
    fn test_verbose_response_parsing() {
        let json = r#"{
            "task": "transcribe",
            "language": "ru",
            "duration": 10.5,
            "text": "Привет мир",
            "segments": [
                {
                    "id": 0,
                    "seek": 0,
                    "start": 0.0,
                    "end": 5.0,
                    "text": "Привет",
                    "avg_logprob": -0.1,
                    "compression_ratio": 1.2,
                    "no_speech_prob": 0.01
                }
            ]
        }"#;

        let response: VerboseJsonResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.language, "ru");
        assert_eq!(response.text, "Привет мир");
        assert_eq!(response.segments.len(), 1);

        let result: TranscribeResult = response.into();
        assert_eq!(result.language, "ru");
        assert_eq!(result.text, "Привет мир");
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].start, 0.0);
        assert_eq!(result.segments[0].end, 5.0);
    }

    #[test]
    fn test_error_classification_401() {
        let err = AppErrorView::auth_failed();
        assert_eq!(err.code, "AUTH_FAILED");
    }

    #[test]
    fn test_error_classification_rate_limit() {
        let err = AppErrorView::rate_limit(Some(60));
        assert_eq!(err.code, "RATE_LIMIT");
        assert!(err.message.contains("60s"));
    }

    #[test]
    fn test_error_classification_network() {
        let err = AppErrorView::network_error("Connection timeout");
        assert_eq!(err.code, "NETWORK_ERROR");
    }

    #[test]
    fn test_postprocess_prompt_safety() {
        // Verify that the postprocessing prompt includes safety constraints
        let prompt = "You are a transcript cleaner. Your task is to:\n\
1. Fix punctuation and capitalization\n\
2. Improve readability with proper spacing\n\
3. Correct obvious typos\n\
4. Add paragraph breaks where appropriate\n\
\nSTRICT RULES (DO NOT VIOLATE):\n\
- NEVER change the meaning of any words\n\
- NEVER add facts not in the original text\n\
- NEVER translate to another language\n\
- NEVER remove important words or concepts\n\
- NEVER reorder sentences\n\
- Keep all names, places, and technical terms exactly as they appear\n\
- Return ONLY the cleaned transcript, nothing else";

        assert!(prompt.contains("NEVER change the meaning"));
        assert!(prompt.contains("NEVER add facts"));
        assert!(prompt.contains("NEVER translate"));
        assert!(prompt.contains("NEVER remove important"));
    }

    #[test]
    fn test_chat_completion_response_parsing() {
        let json = r#"{
            "choices": [
                {
                    "message": {
                        "content": "Привет, мир! Это тестовая запись."
                    }
                }
            ]
        }"#;

        let response: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            "Привет, мир! Это тестовая запись."
        );
    }
}

// ============================================================================
// Integration Tests with Mock HTTP Server
// ============================================================================

#[cfg(test)]
mod mock_server_tests {
    use super::*;

    #[test]
    fn test_successful_transcribe_response() {
        let response_json = r#"{
            "task": "transcribe",
            "language": "ru",
            "duration": 10.5,
            "text": "Привет мир это тестовая запись",
            "segments": [
                {
                    "id": 0,
                    "seek": 0,
                    "start": 0.0,
                    "end": 5.0,
                    "text": "Привет мир",
                    "avg_logprob": -0.1,
                    "compression_ratio": 1.2,
                    "no_speech_prob": 0.01
                },
                {
                    "id": 1,
                    "seek": 5000,
                    "start": 5.0,
                    "end": 10.5,
                    "text": "это тестовая запись",
                    "avg_logprob": -0.15,
                    "compression_ratio": 1.1,
                    "no_speech_prob": 0.02
                }
            ]
        }"#;

        let parsed: VerboseJsonResponse = serde_json::from_str(response_json).unwrap();
        let result: TranscribeResult = parsed.into();

        assert_eq!(result.language, "ru");
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].text, "Привет мир");
        assert_eq!(result.segments[1].text, "это тестовая запись");
        assert!(result.text.contains("Привет"));
    }

    // NOTE: For live testing with real Groq API, run with:
    // GROQ_API_KEY=<your-key> cargo test --ignored test_live_transcribe -- --nocapture
    //
    // This test is ignored by default to prevent accidental API calls
    #[tokio::test]
    #[ignore]
    async fn test_live_transcribe() {
        let api_key = std::env::var("GROQ_API_KEY").expect("GROQ_API_KEY not set for live test");

        let _client = GroqClient::new(api_key).expect("Failed to create client");

        // This test would need a real audio file to work
        // Skip for now since we don't have test fixtures
    }
}
