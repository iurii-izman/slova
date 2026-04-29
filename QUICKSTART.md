# ⚡ VideoTranscriber Quick Start

**For developers who just want to get it running.** See [FIRST-RUN.md](./FIRST-RUN.md) for detailed setup.

## Prerequisites (5 minutes)

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js 18+ from https://nodejs.org

# Install FFmpeg
# macOS: brew install ffmpeg
# Windows: choco install ffmpeg OR winget install ffmpeg
# Linux: apt install ffmpeg
```

Verify:
```bash
rustc --version && node --version && ffmpeg -version
```

## Setup (2 minutes)

```bash
git clone https://github.com/iurii-izman/slova.git
cd slova
cd apps/ui && npm install && cd ../..
```

## Run (1 minute)

**Terminal 1:**
```bash
cd apps/ui && npm run dev
```

**Terminal 2:**
```bash
cargo tauri dev
```

App window opens in 30-60 seconds. UI hot-reloads, Rust recompiles on save.

## Configure API Key (2 minutes)

1. Get free key from https://console.groq.com/keys
2. Open app → Settings (⚙️ icon)
3. Paste key in "🔐 API Key" section
4. Click "Save API Key"
5. Status should show ✓ (green)

## Test

1. Click "Add Files"
2. Select any MP4/MKV/WebM video
3. Wait for transcription (status: Queued → Extracting → Uploading → Transcribing → Done)
4. Click video to see transcript

## Build for Release

```bash
cd src-tauri && cargo tauri build
```

Output: `target/release/bundle/nsis/VideoTranscriber_0.1.0_installer_x64.exe`

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `error: target requires features: with_tauri` | Ensure Cargo.toml has `default = ["with_tauri"]` in `[features]` |
| `Cannot find module @tauri-apps/api/dialog` | Check `apps/ui/package.json` has `@tauri-apps/api: ^2.10.1` |
| `API key invalid` | Key must start with `gsk_` and be copied fully from console.groq.com |
| `FFmpeg not found` | Install FFmpeg and restart app (Windows might need PATH restart) |
| `Port 5173 in use` | Kill old process: `taskkill /F /IM node.exe` (Windows) or `pkill -f vite` (Linux/macOS) |

## Commands Cheatsheet

```bash
# Development
cargo tauri dev                    # Run app in dev mode
cd apps/ui && npm run dev          # UI dev server only (port 5173)
cd apps/ui && npm run check        # TypeScript type check
cd apps/ui && npm run build        # Build UI for production
cd src-tauri && cargo check        # Check Rust compilation
cd src-tauri && cargo test         # Run Rust unit tests
cargo fmt                          # Format all code
cargo clippy                       # Lint check

# Production
cd src-tauri && cargo tauri build  # Create installer

# Debugging
RUST_LOG=debug cargo tauri dev     # Enable debug logging
cd src-tauri && cargo tree         # View dependency tree
```

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/lib.rs` | Rust core: scheduler, pipeline, FFmpeg adapter |
| `src-tauri/src/app/commands.rs` | Tauri IPC commands |
| `apps/ui/src/pages/QueuePage.tsx` | Main queue UI (Solid.js) |
| `apps/ui/src/pages/SettingsPage.tsx` | Settings and API key UI |
| `src-tauri/Cargo.toml` | Backend dependencies |
| `apps/ui/package.json` | Frontend dependencies |

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────┐
│                   UI (Solid.js)                     │
│  • Queue display, drag & drop, settings             │
│  • Real-time progress updates via Tauri events      │
└──────────────────┬──────────────────────────────────┘
                   │ Tauri IPC (invoke + listen)
┌──────────────────▼──────────────────────────────────┐
│             Tauri Runtime (Rust)                    │
│  • Command handlers (enqueue, cancel, etc)         │
│  • Async event emitter                             │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│            Core (Rust + Tokio)                      │
│  ┌────────────────────────────────────────────┐     │
│  │  JobScheduler (Semaphore parallelism)      │     │
│  │  • CPU-bound: FFmpeg extraction (2)        │     │
│  │  • Network: Groq uploads/API (3)           │     │
│  └────────────────────────────────────────────┘     │
│  ┌────────────────────────────────────────────┐     │
│  │  Pipeline State Machine                    │     │
│  │  • Queued → Probing → Extracting           │     │
│  │  • Uploading → Transcribing → Done         │     │
│  └────────────────────────────────────────────┘     │
│  ┌────────────────────────────────────────────┐     │
│  │  Adapters                                  │     │
│  │  • FFmpegAdapter: probe, extract, silence │     │
│  │  • GroqClient: upload, transcribe          │     │
│  │  • KeyringAdapter: OS keychain             │     │
│  │  • SettingsRepo: SQLite persistence        │     │
│  └────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────┘
```

## Performance Baseline

| Task | Time |
|------|------|
| FFmpeg extract (30 min video) | ~5-7 sec |
| Upload 7 MB to Groq | ~1-2 sec |
| Groq Whisper transcription | ~8-15 sec |
| **Total per file** | **~15-25 sec** |
| **5 files in parallel** | **~45-60 sec** |

## Free Tier Limits

- **Groq API:** 30 requests/minute, no credit card required
- **Parallelism:** 3 concurrent jobs recommended (2 extract + 3 upload = safe)
- **File size:** Up to 100 MB audio (after conversion)

## Next

- Full guide: [FIRST-RUN.md](./FIRST-RUN.md)
- Architecture: [transcriber-architecture-analysis.md](./transcriber-architecture-analysis.md)
- Dev plan: [transcriber-autopilot-development-plan.md](./transcriber-autopilot-development-plan.md)

Happy transcribing! 🎬
