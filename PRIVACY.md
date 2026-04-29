# Privacy Policy (Draft for Slova v0.1)

Last updated: 2026-04-29

## What Slova does

Slova is a desktop app that transcribes local video/audio files into text.

## Data processed

### Local data

- Source video/audio file paths
- Transcription results (`.txt/.srt/.json`)
- Job metadata and history in local SQLite database
- Application logs

### Cloud data

When transcription is started, audio content is sent to **Groq API** for speech-to-text processing.

## Secrets and API keys

- Groq API key is stored in OS keychain (Windows Credential Manager / macOS Keychain / Linux Secret Service)
- API key is not stored in source control
- API key must never be shared in chat, issue trackers, or logs

## What is not collected by us

- We do not run our own telemetry backend in v0.1
- We do not upload your local database to our servers

## Third-party processing

Transcription requests are processed by Groq. Use of Groq services is subject to Groq Terms and Privacy Policy.

- Website: https://groq.com
- Console: https://console.groq.com

## User control

You can:

- Remove API key from Settings (`Delete API Key`)
- Delete output transcript files manually
- Delete local app data directory manually (including DB and logs)

## Local storage locations (default)

- App data / DB / logs: OS-specific app data directory for app identifier `com.github.iurii-izman.slova`
- Transcript output files: by default saved next to source files

## Security notes

- Do not commit secrets to repository
- Verify licenses of bundled binaries (ffmpeg/ffprobe/rnnoise model)
- For public distribution, consider code signing

## Contact

For privacy or security concerns, use project issue tracker or security policy in `SECURITY.md`.
