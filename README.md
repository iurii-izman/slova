# Slova

> Fast desktop batch transcription with Groq Whisper (Tauri 2 + Rust + Solid.js)

[![CI](https://github.com/iurii-izman/slova/actions/workflows/tests.yml/badge.svg)](https://github.com/iurii-izman/slova/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Slova is a desktop app for transcribing video/audio files into text with queue processing, retries, and secure API key storage.

## Stack

- **Desktop:** Tauri 2
- **Backend:** Rust + Tokio
- **Frontend:** Solid.js + TypeScript + Vite
- **Storage:** SQLite
- **Secrets:** OS keychain (`keyring`)
- **STT:** Groq Whisper Large v3 Turbo
- **Media:** ffmpeg/ffprobe (+ optional rnnoise `cb.rnnn`)

## Current status

- Queue + job states implemented
- Settings UI + Groq API key save/delete implemented
- Export formats: TXT/SRT/JSON
- Rust unit tests and CI pipeline configured

See:

- `transcriber-spec.md`
- `transcriber-architecture-analysis.md`
- `transcriber-autopilot-development-plan.md`

## Prerequisites

- Rust stable
- Node.js LTS (18+; CI uses 20)
- npm (or pnpm/yarn for local work)
- ffmpeg + ffprobe in PATH

Windows may additionally require:

- WebView2 Runtime
- Visual Studio Build Tools + Windows SDK

## Quick start

1) Install frontend deps:

- `cd apps/ui`
- `npm install`

2) Run in 2 terminals:

- Terminal A: `cd apps/ui && npm run dev`
- Terminal B (repo root): `cargo tauri dev`

3) Open **Settings** in app and save your Groq API key.

## Build (Windows installer)

- `cd apps/ui && npm run build`
- `cargo tauri build`

Artifacts are generated under `src-tauri/target/release/bundle/`.

## API key and privacy

- API key is stored in OS keychain, not in repo
- Audio is sent to Groq API during transcription
- See `PRIVACY.md` and `SECURITY.md`

## rnnoise model (`cb.rnnn`)

Optional, but recommended for noisy audio.

Expected path:

- `resources/rnnoise-models/cb.rnnn`

If missing, Slova continues without noise reduction.

Details: `resources/rnnoise-models/README.md`

## Development quality checks

Frontend:

- `cd apps/ui && npm run check`
- `cd apps/ui && npm run build`

Backend:

- `cd src-tauri && cargo fmt -- --check`
- `cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings`
- `cd src-tauri && cargo test --lib`

## GitHub automation

- CI: `.github/workflows/tests.yml`
- Windows release pipeline: `.github/workflows/release-windows.yml`

## Key docs

- `FIRST-RUN.md` — first-time setup walkthrough
- `GETTING-STARTED.md` — short startup guide
- `BUILD-AND-RUN.md` — build/run commands
- `docs/owner-decisions-v0.1.md` — product decisions for v0.1
- `docs/user-actions-u-block.md` — manual user checklist (U1..U10)

## License

MIT
