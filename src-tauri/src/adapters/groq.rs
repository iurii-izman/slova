// ============================================================================
// Groq API Client
// ============================================================================
// Integration with Groq cloud API for:
// - whisper-large-v3-turbo (speech-to-text)
// - llama-3.1-8b-instant (postprocessing)
// Handles:
// - Request/response serialization
// - Rate limiting (free tier: 30 RPM)
// - Multipart file uploads
// - API key from OS keychain

use crate::types::AppErrorView;

/// HTTP client for Groq API
pub struct GroqClient {
    /// Base URL (usually https://api.groq.com/openai/v1)
    pub base_url: String,
    /// API key from OS keychain (never hardcoded)
    pub api_key: Option<String>,
}

impl GroqClient {
    pub fn new(base_url: String) -> Self {
        GroqClient {
            base_url,
            api_key: None,
        }
    }

    /// Load API key from OS keychain
    pub fn load_api_key(&mut self) -> Result<(), AppErrorView> {
        // TODO: use keyring crate to retrieve from OS keychain
        Err(AppErrorView::auth_failed())
    }

    /// Transcribe audio via Groq Whisper API
    /// - language: "ru"
    /// - response_format: "verbose_json" (includes timings)
    /// - temperature: 0 (deterministic)
    /// - prompt: Russian language hint
    pub async fn transcribe(
        &self,
        _audio_path: &std::path::Path,
        _opts: TranscribeOpts,
    ) -> Result<TranscribeResult, AppErrorView> {
        // TODO: implement multipart upload to Groq API
        Err(AppErrorView::internal_error("transcribe not implemented"))
    }

    /// Postprocess transcript via Groq Llama
    /// Cleans punctuation, grammar, formatting
    pub async fn postprocess(&self, _text: String) -> Result<String, AppErrorView> {
        // TODO: implement
        Err(AppErrorView::internal_error("postprocess not implemented"))
    }
}

impl Default for GroqClient {
    fn default() -> Self {
        Self::new("https://api.groq.com/openai/v1".into())
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Clone, Debug)]
pub struct TranscribeOpts {
    pub language: String,        // e.g. "ru"
    pub temperature: f32,        // 0 for deterministic
    pub prompt: String,          // Language hint
    pub model: String,           // whisper-large-v3-turbo
    pub response_format: String, // verbose_json
}

#[derive(Clone, Debug)]
pub struct TranscribeResult {
    pub text: String,
    pub segments: Vec<TranscriptSegmentResult>,
}

#[derive(Clone, Debug)]
pub struct TranscriptSegmentResult {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}
