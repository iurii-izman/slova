# 🚀 VideoTranscriber First Run Guide

Welcome! This guide walks you through setting up and using VideoTranscriber for the first time.

## Step 1: Install Prerequisites

Before you can run VideoTranscriber, you need to install some required software:

### A. Install Rust (Backend Compiler)
1. Go to https://rustup.rs
2. Copy the command and run it in your terminal
3. Follow the prompts (accept defaults)
4. Restart your terminal to apply changes
5. Verify: `rustc --version` should show a version number

### B. Install Node.js (Frontend Runtime)
1. Go to https://nodejs.org
2. Download and install **LTS version** (currently 20.x or higher)
3. Verify: `node --version` should show a version number

### C. Install FFmpeg (Audio Conversion Tool)
This is essential for converting videos to audio for transcription.

**Windows:**
```powershell
# Option 1: Using Chocolatey (easiest)
choco install ffmpeg

# Option 2: Using Windows Package Manager
winget install ffmpeg

# Option 3: Manual download
# Visit https://ffmpeg.org/download.html
# Extract to C:\ffmpeg
# Add C:\ffmpeg\bin to your Windows PATH
```

**macOS:**
```bash
brew install ffmpeg
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt install ffmpeg
```

**Verify installation:**
```bash
ffmpeg -version  # Should show version info
ffprobe -version # Should show version info
```

## Step 2: Clone the Repository

```bash
# Clone the code
git clone https://github.com/iurii-izman/slova.git
cd slova

# Install frontend dependencies
cd apps/ui
npm install
cd ../..
```

## Step 3: Get a Free Groq API Key

The app uses Groq's API to transcribe videos. The free tier gives you 30 requests per minute, which is enough for batch transcription.

### A. Create Groq Account
1. Go to https://console.groq.com
2. Click "Sign Up"
3. Choose your preferred sign-up method (Google, GitHub, or email)
4. Verify your email address

### B. Generate API Key
1. Once logged in, go to https://console.groq.com/keys
2. Click "Create New Secret Key"
3. Give it a name like "VideoTranscriber"
4. Copy the key (it looks like: `gsk_xxxxxxxxxxxxxxxxxxxx`)
5. ⚠️ **Save it somewhere safe** — you won't be able to see it again

### C. Free Tier Limits
- **30 requests per minute** — perfect for batch transcription
- **Sufficient for:** 5-10 videos processing in parallel
- **Cost:** Completely free (no credit card required)

## Step 4: Run the App in Development Mode

Open **two terminal windows** from the `slova` directory:

**Terminal 1 — Frontend:**
```bash
cd apps/ui
npm run dev
```
This starts the Vite development server on `http://localhost:5173`

**Terminal 2 — Backend (from slova root directory):**
```bash
cargo tauri dev
```

Wait for the app window to appear (might take 30-60 seconds on first run).

You'll see:
- Terminal 1: Vite server ready
- Terminal 2: App window opens automatically

**Why two terminals?**
- Changes to UI files reload in ~100ms
- Changes to Rust files recompile in ~2-5 seconds
- Running in parallel keeps both fast

## Step 5: Add Your API Key

Now that the app is running, you need to configure your Groq API key:

1. **Open Settings**
   - Click the ⚙️ (settings) icon in the top-right corner
   - Or press `Ctrl+,` (or `Cmd+,` on macOS)

2. **Navigate to API Key Section**
   - You'll see a section labeled "🔐 API Key"
   - It should say "✗ No API key set"

3. **Paste Your API Key**
   - Click the input field
   - Paste your Groq API key (from Step 3)
   - The field should show dots as you type (password-style)

4. **Save the Key**
   - Click "Save API Key" button
   - You should see a green toast: "API key saved successfully"
   - The status should change to "✓ API key is configured"

## Step 6: Test with a Sample Video

Now let's test the transcription:

### A. Add a Video
1. Go back to the main queue
2. Click "Add Files" button (or drag & drop a video)
3. Select an MP4, MKV, or WebM file
4. Video should appear in the queue with status "Queued"

### B. Monitor Progress
- **Status changes:**
  - Queued → Extracting (converting audio) → Uploading → Transcribing → Done
- **Progress bars** show percentage for each stage
- **Elapsed time** shows how long each stage took

### C. View Results
- Once status reaches "Done", click the video
- Transcript appears in the detail panel
- You can edit text, copy to clipboard, or export

## Step 7: Configure Your Preferences

Back in Settings, you can customize:

### Processing Options
- **Language:** Select the language of your videos (default: Russian)
- **Output Format:** TXT, SRT (subtitles), or JSON
- **Concurrent Jobs:** How many videos to process simultaneously
  - 1-3 recommended for stability
  - Higher = faster but uses more bandwidth/CPU
- **Enable Postprocessing:** Optional grammar cleanup (adds ~1 sec per file)

## Step 8: Batch Processing

Ready to transcribe multiple videos?

1. **Add multiple files**
   - Click "Add Files" and select 5-10 videos
   - Or drag-drop multiple files at once

2. **Adjust parallelism if needed**
   - Go to Settings
   - Set "Concurrent Jobs" to 2-3 (Groq free tier is 30 RPM)

3. **Let it run**
   - App will process in parallel
   - You can see progress for each file

## Troubleshooting

### "API key is invalid or not set"
- **Solution:** Check that your API key starts with `gsk_`
- Make sure you copied the entire key from console.groq.com
- Restart the app after saving

### "FFmpeg not found"
- **Windows:** `choco install ffmpeg` or add FFmpeg to your PATH
- **macOS:** `brew install ffmpeg`
- **Linux:** `sudo apt install ffmpeg`
- Restart the app

### App crashes on startup
- **Check logs:** Open Settings → click "View Logs" folder
- **Enable debug mode:**
  ```bash
  # Windows (PowerShell)
  $env:RUST_LOG="debug"
  cargo tauri dev
  
  # macOS/Linux
  RUST_LOG=debug cargo tauri dev
  ```

### Videos transcribe very slowly or fail
- Check your internet connection
- Check Groq API status: https://status.groq.com
- Try a smaller video file to test
- Check file is valid MP4/MKV: `ffprobe "your_video.mp4"`

### "Port 5173 already in use"
- Kill the previous process:
  ```bash
  # Windows
  taskkill /F /IM node.exe
  
  # macOS/Linux
  pkill -f "vite"
  ```
- Then run `cargo tauri dev` again

## Next Steps

Once you're comfortable with transcription:

1. **Organize outputs** — All `.txt` files are saved next to source videos
2. **Explore export formats** — Try SRT for subtitle timing or JSON for timestamps
3. **Batch large libraries** — Process 20+ videos overnight with Settings → Concurrent Jobs set to 2
4. **Edit transcripts** — Click videos to edit text inline before exporting

## Getting Help

- 📖 **Full Documentation:** See [README.md](./README.md)
- 🏗️ **Architecture Details:** See [transcriber-architecture-analysis.md](./transcriber-architecture-analysis.md)
- 🐛 **Report Issues:** https://github.com/iurii-izman/slova/issues
- 🔒 **Security Info:** See [SECURITY.md](./SECURITY.md)

## Common Commands Reference

```bash
# Development (run in two terminals)
# Terminal 1:
cd apps/ui && npm run dev           # UI hot-reload on port 5173

# Terminal 2 (from project root):
cargo tauri dev                     # Backend + app window

# Build for production
cd apps/ui && npm run build         # Build UI
cargo tauri build                   # Create installer

# Testing & Quality
cd src-tauri && cargo test          # Run backend tests
cd apps/ui && npm run check         # TypeScript check
cargo fmt                           # Format all code
cargo clippy                        # Lint check

# Debugging
RUST_LOG=debug cargo tauri dev      # Enable detailed logging
```

## Performance Tips

- **Encoding:** Opus 32kbps mono saves 70% bandwidth vs MP3
- **Processing:** 3 concurrent jobs = ~30 seconds per file on Groq free tier
- **Storage:** 30-minute video → ~500 KB transcript file
- **Network:** Upload speed is critical; check with `speedtest.net`

Good luck with your transcriptions! 🎉
