# Build and Run Guide

## Prerequisites (One-time setup)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js 18+ from https://nodejs.org

# Install FFmpeg
# Windows: choco install ffmpeg  
# macOS: brew install ffmpeg
# Linux: apt install ffmpeg

# Verify
rustc --version && node --version && ffmpeg -version
```

## Clone & Setup

```bash
git clone https://github.com/iurii-izman/slova.git
cd slova
cd apps/ui && npm install && cd ../..
```

## Development Mode

**Terminal 1 (Frontend hot-reload):**
```bash
cd apps/ui
npm run dev
```

**Terminal 2 (Backend + App):**
```bash
cargo tauri dev
```

App opens on http://localhost:5173 with hot-reload.

## Production Build

```bash
cd apps/ui
npm run build

cargo tauri build
```

Output: `target/release/bundle/nsis/VideoTranscriber_0.1.0_installer_x64.exe`

## Quick Commands

```bash
# Type checking
cd apps/ui && npm run check

# Run tests
cd src-tauri && cargo test --lib

# Format code
cargo fmt

# Lint check
cargo clippy
```

## First Time? 

See [FIRST-RUN.md](./FIRST-RUN.md) for step-by-step setup guide including how to get and configure your Groq API key.

## Troubleshooting

**"FFmpeg not found"** → Restart terminal after installing  
**"Port 5173 in use"** → Kill old process: `taskkill /F /IM node.exe`  
**"API key invalid"** → Key must start with `gsk_` from https://console.groq.com/keys  
**App crashes?** → Check logs: Settings → "View Logs" or `RUST_LOG=debug cargo tauri dev`
