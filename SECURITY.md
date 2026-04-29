# Security & Logging Policy

## 🔒 Security-First Design

This document describes security measures implemented in VideoTranscriber.

### API Key Management

**Secure Storage:**
- API keys are stored **exclusively in OS keychain**, never in code or database
- Windows: Windows Credential Manager
- macOS: Keychain
- Linux: Secret Service (via `keyring` crate)

**Access:**
```rust
// Loading API key
let api_key = KeyringAdapter::get_api_key()?;

// Saving API key (called from Settings UI)
KeyringAdapter::save_api_key(&key)?;
```

**Never Logged:**
- API key itself is never written to logs or console
- Only confirmation message is logged: "API key saved to OS keychain successfully"
- If keyring fails, logged as "Keyring error when loading API key" (generic message, no details exposed)

### Sensitive Data Protection

**What is NOT Logged:**
- Full API request/response bodies (only metadata like status code, duration)
- Full transcript text at DEBUG level
- User API keys or credentials
- Full file paths in error messages (only filename)

**What IS Logged (at INFO level):**
- Job state transitions: `Queued → Probing → Extracting → Transcribing → Done`
- API request metadata: duration, status code, retry count
- Successful file operations: "Transcript saved to /path/to/file.txt"
- System events: app startup, shutdown, recovery

### Input Validation

All external inputs are validated:
- **File paths:** Must exist, be readable, end in `.mp4`
- **Settings:** Parallelism must be 1-10, language code validated against allowed list
- **API key:** Must be 20+ characters (basic sanity check)
- **Transcript edits:** Cannot be empty

### Process Security

FFmpeg and FFprobe are executed with argument arrays, not shell commands:
```rust
// SAFE: Uses process API, not shell execution
let output = Command::new("ffmpeg")
    .args(&["-i", input_path, "-vn", "-ac", "1", output_path])
    .output()?;

// UNSAFE (not used): Shell injection risk
// os::system(format!("ffmpeg -i {} {}", input, output))
```

### Content Security Policy (CSP)

The app enforces a strict CSP in `tauri.conf.json`:
```json
{
  "csp": "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https://api.groq.com; frame-ancestors 'none';"
}
```

This prevents:
- Loading scripts from CDNs
- Inline style execution (except necessary `unsafe-inline` for Solid.js)
- Framing the app in iframes

### Tauri Capabilities

Only necessary capabilities are enabled (restrictive by default):
- `fs:allow-read` — Read video files and log directory
- `fs:allow-write` — Write transcript files and logs
- `shell:allow-execute` — Execute ffmpeg/ffprobe only (sidecar restriction recommended)
- `app:allow-app-show` — Window management
- `core:allow-internal-invoke` — Backend communication

Future versions will use `@tauri-apps/api` capabilities for finer-grained control.

---

## 📊 Logging & Diagnostics

### Log Storage

**Locations by Platform:**
- **Windows:** `%APPDATA%\Roaming\slova\logs\`
- **macOS:** `~/Library/Application Support/slova/logs/`
- **Linux:** `~/.local/share/slova/logs/`

**File Format:**
- Daily rotation: `transcriber.log.2024-01-15`, `transcriber.log.2024-01-16`, etc.
- Text format with timestamps and thread IDs
- Each line: `TIMESTAMP [LEVEL] [MODULE] MESSAGE`

**Example Log Entry:**
```
2024-01-15T14:32:01.234Z INFO slova_tauri::app::scheduler: Job enqueued: job_id=550e8400-e29b-41d4-a716-446655440000
2024-01-15T14:32:02.345Z INFO slova_tauri::core::pipeline: Stage transition: stage=Probing, job_id=550e8400-e29b-41d4-a716-446655440000
2024-01-15T14:32:15.678Z INFO slova_tauri::adapters::groq: Transcription complete: duration_ms=13000, chunks=1
```

### Log Levels

| Level | Usage | When to Enable |
|-------|-------|-----------------|
| **TRACE** | Function entry/exit, variable state | Almost never (extreme debugging) |
| **DEBUG** | Detailed operation steps, buffer contents | Development, troubleshooting specific module |
| **INFO** | Job state transitions, API calls, recovery events | Default (production) |
| **WARN** | Recoverable errors, retries, rate limits | Always shown (default) |
| **ERROR** | Critical failures, panics, uncaught errors | Always shown (default) |

### Enabling Debug Logging

**Environment Variable (before startup):**
```bash
# Windows PowerShell
$env:RUST_LOG="debug"
cargo run --features with_tauri

# Linux/macOS
export RUST_LOG=debug
cargo run --features with_tauri
```

**Specific Modules:**
```bash
# Only scheduler module
RUST_LOG="slova_tauri::core::scheduler=debug" cargo run --features with_tauri

# Multiple modules with different levels
RUST_LOG="slova_tauri=debug,tokio=info,hyper=warn" cargo run --features with_tauri
```

### Structured Logging Features

**Job Context Spans:**
Each job operation is logged with context:
```rust
#[tracing::instrument(skip(state), fields(job_id = %job_id, stage = "extracting"))]
pub async fn extract_audio(job_id: JobId, path: &Path, state: &AppState) -> Result<()> {
    tracing::info!("Starting audio extraction");
    // ...
}
```

Logs will include:
```
job_id=550e8400-e29b-41d4-a716-446655440000 stage=extracting Starting audio extraction
```

**Panic Hook:**
If the app panics, backtrace is logged:
```
ERROR slova_tauri: Application panic
panic='attempt to divide by zero'
backtrace=...
```

### In-App Log Access

**From Settings UI:**
- Button: "View Logs" → Opens logs folder in file explorer
- Button: "Copy Last 100 Lines" → Clipboard for sharing

**Via Backend API:**
```javascript
// Get last 100 log lines from frontend
const logs = await invoke('get_logs', { lines: 100 });

// Open logs folder
await invoke('open_logs_folder');
```

---

## 🔄 Startup Recovery

### Automatic Job Recovery

When the app starts, it automatically recovers jobs that were in progress:

1. **Check Database:** Query all jobs in non-terminal states (Queued, Probing, Extracting, etc.)
2. **Re-enqueue:** Each active job is added back to the scheduler queue
3. **Log:** "Recovered 5 active jobs from database"

**Job States (recoverable):**
- `Queued` — Not yet started
- `Probing` — Checking file validity
- `Extracting` — Converting video to audio
- `Chunking` — Splitting large files
- `Uploading` — Sending to Groq API
- `Transcribing` — Waiting for Groq response
- `Stitching` — Combining chunks
- `Postprocessing` — Llama cleanup (optional)

**Job States (final, NOT recovered):**
- `Done` — Successfully completed
- `Failed` — Error occurred (user must retry manually)
- `Cancelled` — User cancelled
- `Paused` — User paused (will be resumed if queue was running)

### Pause/Resume State

If the app is closed while queue is paused:
- Paused jobs are recovered in paused state (not restarted)
- Resume button available in UI to continue
- This prevents accidental re-processing of paused jobs

### Retry with Backoff

If a job fails:
1. Logged as: "Job failed: job_id=..., error=RATE_LIMIT, attempts=1"
2. Exponential backoff calculated: `min(2^attempt * 100ms + jitter, 30s)`
3. Job automatically retried (up to 3 times by default)
4. If all retries exhausted, job state becomes `Failed`

Example retry sequence for rate limit:
```
Attempt 1: Try, get 429, wait 100-200ms
Attempt 2: Try, get 429, wait 400-600ms
Attempt 3: Try, get 429, wait 900-1100ms
Failed: User sees error in UI, can click "Retry" button
```

---

## 🧪 Testing Security

### Unit Test Coverage

```bash
cd src-tauri
cargo test -- --nocapture
```

Tests verify:
- API key is not logged
- Error messages don't expose paths
- File validation rejects invalid inputs
- Process execution uses safe APIs

### Manual Security Audit

1. **Check logs don't contain API key:**
   ```bash
   grep -r "sk-" ~/.local/share/slova/logs/  # Should be empty
   ```

2. **Check logs don't contain full transcript:**
   ```bash
   grep -r "text_from_groq_api" ~/.local/share/slova/logs/  # Should be empty
   ```

3. **Check keyring storage works:**
   - Save API key from UI
   - Restart app
   - Verify app initializes successfully (logs show "API key loaded from keyring")
   - Check keyring doesn't have plaintext on disk

4. **Check CSP is enforced:**
   - Open DevTools (F12 in dev build)
   - Check console for CSP violations
   - Try to load external stylesheet → should fail with CSP error

---

## 🛠️ Development Best Practices

### When Adding New Features

**DO:**
- ✅ Log state transitions at INFO level
- ✅ Use structured logging with `#[tracing::instrument]` for spans
- ✅ Validate all user inputs
- ✅ Use `AppErrorView` for errors (not string errors)
- ✅ Handle sensitive data: don't log API keys, tokens, full paths
- ✅ Use process API instead of shell commands

**DON'T:**
- ❌ Log API keys or user credentials
- ❌ Use `println!()` (use `tracing::info!()` instead)
- ❌ Execute shell commands with string interpolation
- ❌ Expose internal errors to user (map to user-friendly `AppErrorView`)
- ❌ Store secrets in environment variables or config files
- ❌ Commit `.env` files or API keys to git

### Error Handling Pattern

```rust
// Good: Type-safe, user-friendly
pub async fn transcribe(job_id: JobId, state: &AppState) -> Result<(), AppErrorView> {
    match state.groq.transcribe(&file).await {
        Ok(text) => Ok(()),
        Err(GroqError::RateLimit { retry_after }) => {
            Err(AppErrorView::rate_limit(Some(retry_after)))
        }
        Err(GroqError::AuthFailed) => {
            Err(AppErrorView::auth_failed())
        }
        Err(e) => {
            tracing::error!("Unexpected Groq error: {}", e); // Log internal details
            Err(AppErrorView::internal_error("Failed to transcribe"))
        }
    }
}

// Bad: Stringly-typed, exposes internals
pub async fn transcribe(job_id: JobId, state: &AppState) -> Result<String, String> {
    match state.groq.transcribe(&file).await {
        Ok(text) => Ok(text),
        Err(e) => {
            println!("ERROR: {}", e); // Logged as plaintext
            Err(e.to_string()) // User sees raw error
        }
    }
}
```

---

## 📋 Supported Versions

| Version | Supported | Status |
|---------|-----------|--------|
| 0.1.x   | ✅        | Active development |
| < 0.1   | ❌        | N/A |

---

## 🚨 Reporting Security Issues

If you discover a security vulnerability:

1. **Do NOT create a public GitHub issue**
2. **Email:** [security@iurii-izman.dev](mailto:security@iurii-izman.dev) (replace with actual if available)
3. **Include:**
   - Description of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (optional)

We will:
- Acknowledge receipt within 48 hours
- Provide detailed response within 7 days
- Work with you to fix the issue before public disclosure
- Credit you in release notes if you wish

---

## References

- [OWASP: Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
- [Rust: Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Tauri: Security Best Practices](https://tauri.app/en/v1/guides/dist-tauri/security/)
- [tracing-rs: Structured Logging](https://docs.rs/tracing/latest/tracing/)
