# Groq Network Layer Implementation

## Overview

Production-ready Groq API client for VideoTranscriber implementing:
- Multipart audio file upload to `/openai/v1/audio/transcriptions`
- Verbose JSON response deserialization (with segments and timings)
- Rate limiting (30 RPM, free tier)
- Retry/backoff with exponential backoff and jitter
- Error classification and non-retryable vs transient error handling
- Secure API key management (OS keyring)

## Architecture

### GroqClient Structure

```rust
pub struct GroqClient {
    http: Client,                                           // reqwest client
    _api_key: SecretString,                                 // Never logged
    base_url: String,                                       // https://api.groq.com/openai/v1
    rate_limiter: Arc<parking_lot::Mutex<RateLimiter>>,   // 30 RPM limiter
}
```

### Rate Limiting

**Token-bucket algorithm:**
- Max tokens: 30 (30 requests per minute, free tier limit)
- Refill rate: 0.5 tokens/second (30 / 60)
- Async-safe with `parking_lot::Mutex` for low-latency locking

```rust
struct RateLimiter {
    max_tokens: f64,      // 30.0
    tokens: f64,          // Current balance
    last_refill: f64,     // seconds since epoch
    refill_rate: f64,     // 0.5 tokens/sec
}
```

### Retry Strategy & Error Classification

**Manual retry loop** (no external crate dependency):

| Error Class | HTTP Status | Retry? | Strategy |
|---|---|---|---|
| **Auth** | 401 | ❌ | Immediate failure, no retry |
| **Rate Limit** | 429 | ✅ | Sleep for `Retry-After` header (or default 60s), max 5 attempts |
| **Server** | 5xx | ✅ | Exponential backoff: 1s → 2s → 4s → 8s → 16s, max 5 attempts |
| **Network** | timeout, connect | ✅ | Exponential backoff, max 5 attempts |
| **Other 4xx** | 400, 403, etc | ❌ | Immediate failure, API error returned |
| **Parsing** | 200 but bad JSON | ❌ | Immediate failure |

**Backoff formula:**
```
delay = min(2^attempt, 32) seconds
max_total_time = 120 seconds
```

### Multipart Upload

Uses `reqwest::multipart::Form` with:
- Model: `whisper-large-v3-turbo`
- Language: `ru` (configurable)
- Response format: `verbose_json` (returns segments with timings)
- Temperature: `0` (deterministic)
- Prompt: "Это запись на русском языке. Говорит один человек." (configurable)

**File handling:**
- Reads entire audio file into memory (acceptable for ≤100MB)
- Clones for each retry attempt (stateless for retry logic)
- Filename preserved in multipart form

## API Design

### Main Entry Points

#### `GroqClient::new(api_key: String) -> Result<Self, AppErrorView>`

Creates client with:
- API key validation (non-empty)
- Bearer token header setup
- HTTP client with 5-minute timeout
- Rate limiter initialization

#### `async fn transcribe(&self, audio_path: &Path, opts: TranscribeOpts) -> Result<TranscribeResult, AppErrorView>`

Core transcription method:
1. Validates audio file exists and is not empty
2. Enters retry loop (max 5 attempts, 2-minute max elapsed)
3. Acquires rate-limit token before each request
4. Builds multipart form
5. Sends POST to `/audio/transcriptions`
6. Parses `VerboseJsonResponse` into `TranscribeResult`

#### `async fn postprocess(&self, text: String) -> Result<String, AppErrorView>`

Placeholder for Groq Llama-based postprocessing (future implementation).

### Types

#### TranscribeOpts

```rust
pub struct TranscribeOpts {
    pub language: String,        // e.g. "ru"
    pub temperature: f32,        // 0 for deterministic
    pub prompt: String,          // Language hint
    pub model: String,           // whisper-large-v3-turbo
    pub response_format: String, // verbose_json
}
```

Default values:
- Language: `"ru"`
- Temperature: `0.0`
- Prompt: "Это запись на русском языке. Говорит один человек."
- Model: `"whisper-large-v3-turbo"`
- Format: `"verbose_json"`

#### TranscribeResult

```rust
pub struct TranscribeResult {
    pub text: String,                           // Full transcription
    pub language: String,                       // Detected language
    pub segments: Vec<TranscriptSegmentResult>, // Timeline segments
}

pub struct TranscriptSegmentResult {
    pub id: u32,
    pub start: f32,            // seconds
    pub end: f32,              // seconds
    pub text: String,          // Segment text
    pub temperature: f32,      // Always 0.0
    pub avg_logprob: f32,      // Confidence metric
    pub compression_ratio: f32, // Audio compression ratio
    pub no_speech_prob: f32,    // Silence probability
}
```

## Error Handling

All errors are mapped to `AppErrorView` with structured codes:

```rust
pub struct AppErrorView {
    pub code: String,           // e.g. "RATE_LIMIT", "AUTH_FAILED"
    pub message: String,        // Human-readable message
    pub details: Option<String>, // Extra context (HTTP status, etc)
}
```

**Error codes:**
- `AUTH_FAILED` - Invalid/missing API key
- `RATE_LIMIT` - 429, includes Retry-After suggestion
- `NETWORK_ERROR` - Connection/timeout issues
- `API_ERROR` - Groq API error (4xx)
- `FS_ERROR` - File system errors
- `INVALID_FILE` - Empty or corrupt audio file
- `INTERNAL_ERROR` - Parsing, multipart construction, etc

## Security

### API Key Management

- Never passed as string literal anywhere
- Stored in `secrecy::SecretString` to prevent accidental logging
- Never logged in debug/trace output
- Loaded from OS keyring via `KeyringAdapter`

### No Shell Injection

- All parameters passed via HTTP multipart form
- File data passed as bytes (no path interpolation)
- No `ProcessBuilder` or shell command execution

## Testing

### Unit Tests

Located in `src/adapters/groq.rs`:

1. **test_rate_limiter_creation** - Rate limiter initializes with 30 tokens
2. **test_groq_client_new** - Valid API key accepted
3. **test_groq_client_empty_key** - Empty key rejected
4. **test_transcribe_opts_default** - Default options use Russian, deterministic, Whisper Large
5. **test_verbose_response_parsing** - JSON deserialization works correctly
6. **test_error_classification_401** - 401 maps to AUTH_FAILED
7. **test_error_classification_rate_limit** - 429 maps to RATE_LIMIT
8. **test_error_classification_network** - Network errors mapped correctly

**Run tests:**
```bash
cargo test adapters::groq
```

### Mock Tests

**test_successful_transcribe_response** - Validates verbose_json response parsing with two segments.

**test_live_transcribe** (ignored by default):
```bash
GROQ_API_KEY=<your-key> cargo test --ignored test_live_transcribe -- --nocapture
```

Note: Live test requires actual audio file fixtures (not included in repo).

## Integration Points

### With Scheduler

`JobScheduler` will:
1. Call `GroqClient::transcribe()` within job state machine
2. Pass audio from `extracting` state
3. Handle returned `TranscribeResult` for `stitching` state
4. Map errors to `JobState::Failed`

### With UI

Return type serializable via `serde_json`:
- `TranscribeResult` → JSON for Solid.js components
- `AppErrorView` → Structured error display
- Segments enable timeline scrubbing and editing

### With Settings

`TranscribeOpts` populated from `Settings`:
```rust
TranscribeOpts {
    language: settings.language,
    prompt: settings.prompt_hint,
    model: settings.groq_model,
    temperature: 0.0, // Always deterministic
    response_format: "verbose_json".into(),
}
```

## Performance Notes

- **First request:** ~1-2 sec (HTTP overhead + encode)
- **Groq API time:** ~8-15 sec per 30 minutes audio
- **Rate limit:** 30 RPM means 2 concurrent requests max under sustained load
- **Backoff:** Worst case (5 retries, exponential): ~30 seconds total

## Future Enhancements

1. **Progress callback** - Track upload bytes (currently read entire file first)
2. **Chunking for >100MB** - Split using silence detection before upload
3. **Stream response** - Use streaming decoder for large responses
4. **Postprocess integration** - Llama-based grammar/punctuation cleanup
5. **Caching** - Cache by file SHA256 to avoid reprocessing
6. **Metrics** - Track success rates, avg latency, rate limit hits
