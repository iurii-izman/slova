# Slova

> Windows-first desktop batch transcription with Groq Whisper, queueing, and local transcript editing.

[![CI](https://github.com/iurii-izman/slova/actions/workflows/tests.yml/badge.svg)](https://github.com/iurii-izman/slova/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)](https://github.com/iurii-izman/slova)
[![Stack: Tauri 2](https://img.shields.io/badge/stack-Tauri%202%20%2B%20Rust%20%2B%20Solid.js-24C8DB)](https://github.com/iurii-izman/slova)

Slova is a portfolio-grade transcription desktop app for turning video or audio
files into text with queue processing, retry logic, export formats, and secure
API key handling.

## Public Review Path

- [Quick start](QUICKSTART.md)
- [Privacy boundary](PRIVACY.md)
- [Release instructions](RELEASE-INSTRUCTIONS.md)
- [Packaging notes](PACKAGING.md)
- [Screenshots plan](docs/screenshots_plan.md)

## What It Demonstrates

- Desktop product architecture with a Rust backend and typed UI boundary.
- File-based transcription workflow with queue states and deterministic retries.
- Local persistence, export formats, and secure secret handling.
- Public-repo readiness: CI, tests, release workflow, and setup docs.

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
- Export formats: TXT / SRT / JSON
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

## Quick Start

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

## API Key and Privacy

- API key is stored in OS keychain, not in repo
- Audio is sent to Groq API during transcription
- See `PRIVACY.md` and `SECURITY.md`

## rnnoise model (`cb.rnnn`)

Optional, but recommended for noisy audio.

Expected path:

- `resources/rnnoise-models/cb.rnnn`

If missing, Slova continues without noise reduction.

Details: `resources/rnnoise-models/README.md`

## Development Quality Checks

Frontend:

- `cd apps/ui && npm run check`
- `cd apps/ui && npm run build`

Backend:

- `cd src-tauri && cargo fmt -- --check`
- `cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings`
- `cd src-tauri && cargo test --lib`

## GitHub Automation

- CI: `.github/workflows/tests.yml`
- Windows release pipeline: `.github/workflows/release-windows.yml`

## Key Docs

- `FIRST-RUN.md` — first-time setup walkthrough
- `GETTING-STARTED.md` — short startup guide
- `BUILD-AND-RUN.md` — build/run commands
- `docs/owner-decisions-v0.1.md` — product decisions for v0.1
- `docs/user-actions-u-block.md` — manual user checklist (U1..U10)

## License

MIT
