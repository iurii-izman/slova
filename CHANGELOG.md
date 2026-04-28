# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for Cycle 3

- Frontend UI implementation with Solid.js
- Real-time queue updates via Tauri events
- Fallback chunking for files >100MB
- Export formats (SRT, JSON)
- Mock Groq API for testing
- Transcript editor UI

## [0.2.0-alpha] - 2024-04-29

### Added — Cycle 2: Core Scheduler & E2E Pipeline ✅ Complete

**5 New Core Modules (~940 lines):**
- `core/cancellation.rs` — Cancellation tokens with async wait support (141 lines, 4 tests)
- `core/progress.rs` — Event broadcasting and tick batching (189 lines, 2 tests)
- `core/retry.rs` — Exponential backoff with jitter 100ms-30s (170 lines, 5 tests)
- `core/stages.rs` — Pipeline stage definitions (220 lines, 1 test)
- `core/pipeline.rs` — State machine executor with error handling (224 lines, 1 test)

**Job Scheduler Implementation:**
- `JobScheduler` with FIFO queue and semaphore-based parallelism
- CPU semaphore (2 concurrent) for ffmpeg/ffprobe operations
- Network semaphore (3 concurrent) for Groq API (respects 30 RPM free tier)
- Cancellation support for individual jobs and batch operations
- Pause/resume functionality for entire queue

**State Machine Pipeline:**
- Complete transitions: Queued → Probing → Extracting → Uploading → Transcribing → Done/Failed
- SQLite persistence for all state changes
- Automatic retry with exponential backoff for transient errors
- Type-safe error classification (Retryable vs Permanent)
- Progress event broadcasting for UI integration

**App Integration:**
- Full `AppState` initialization with all components
- 10 Tauri commands fully implemented with DB persistence:
  - Queue: `enqueue_files()`, `list_jobs()`, `cancel_job()`, `retry_job()`, `pause_queue()`, `resume_queue()`
  - Transcript: `get_transcript()`, `save_transcript_edit()`, `export()`
  - Admin: `save_api_key()`, `get_settings()`, `set_settings()`, `health_check()`
- Event system ready for frontend subscription

**Testing & Quality:**
- 38/38 unit tests passing ✅
- Zero compilation errors
- 100% test pass rate
- cargo fmt, cargo check, cargo clippy all green

**Documentation:**
- `CYCLE-2-COMPLETION.md` — Detailed implementation report with architecture
- `QUICKSTART-PIPELINE.md` — API reference and usage examples
- `PROJECT_STATUS.md` — Development status and metrics

### Changed — Cycle 2

- Updated `JobScheduler` implementation (was skeleton, now fully functional)
- Refactored `AppState` for proper component initialization
- Rewrote all Tauri commands with real database integration
- Enhanced `Cargo.toml` with dashmap, updated tokio features
- Updated `tauri.conf.json` for proper development configuration
- Improved `build.rs` with proper PNG icon generation

### Fixed — Cycle 2

- Fixed async Send safety by replacing `parking_lot::Mutex` with `tokio::sync::Mutex` in Groq client
- Corrected PNG icon CRC calculation in build script
- Fixed test assertions for exponential backoff with jitter tolerance
- Resolved tauri.conf.json schema validation issues

### Security — Cycle 2

- All API keys remain in OS keychain (no database storage)
- Error messages don't leak sensitive information
- Input validation on file paths
- SQL injection protection via parameterized queries

## [0.1.0] - 2024-04-28

### Added — Phase 1: Persistence Layer ✅ Complete

**Database Layer:**
- SQLite database with automatic migrations
- 4 data tables: `jobs`, `transcripts`, `cache`, `settings`
- Repository pattern for type-safe data access
- Async repository implementations:
  - `JobRepo` — Job CRUD, state management, filtering
  - `TranscriptRepo` — Transcript storage and editing
  - `CacheRepo` — File deduplication by content hash
  - `SettingsRepo` — Key-value settings store
- Full test coverage for all repository operations

**Secure Secrets Management:**
- OS Keychain integration via `keyring` crate
- Platform support: Windows (Credential Manager), macOS (Keychain), Linux (Secret Service)
- Safe API key storage (never logged or stored in database)
- Methods: `save_api_key()`, `get_api_key()`, `delete_api_key()`

**Type System:**
- Domain types with full serialization:
  - `Job` — Task representation with metadata
  - `JobState` — Detailed state machine with progress tracking
  - `AppErrorView` — Typed error handling for frontend
  - `JobFilter`, `ExportFormat`, `Settings` — Configuration types
- UUID-based job identification for uniqueness

**Tauri Commands (Skeleton):**
- IPC handlers defined for all planned operations:
  - Queue management: `enqueue_files()`, `list_jobs()`, `cancel_job()`, `retry_job()`, `pause_queue()`, `resume_queue()`
  - Transcript operations: `get_transcript()`, `save_transcript_edit()`, `export()`
  - Settings: `save_api_key()`, `get_settings()`, `set_settings()`
  - Health check: `health_check()`, `emit_demo_event()`

**Frontend Integration:**
- Solid.js + TypeScript setup with Vite
- Tauri API integration
- Ready for queue store implementation
- Component structure prepared

**Documentation:**
- Technical specification (`transcriber-spec.md`)
- Detailed architecture analysis (`transcriber-architecture-analysis.md`)
- Development plan with AI-friendly blocks (`transcriber-autopilot-development-plan.md`)
- Setup and testing guides

### Architecture — Phase 1

- **Layered architecture:**
  - Types layer (domain models)
  - Database layer (SQLite + repositories)
  - Adapters layer (external services)
  - Core layer (business logic, Phase 2 ✅)
  - App layer (Tauri IPC handlers)
  - Frontend layer (Solid.js UI, Phase 3 planned)

- **Security-first design:**
  - API keys in OS keychain
  - Type-safe error handling
  - Input validation
  - No hardcoded secrets

[Unreleased]: https://github.com/iurii-izman/slova/compare/v0.2.0-alpha...HEAD
[0.2.0-alpha]: https://github.com/iurii-izman/slova/compare/v0.1.0...v0.2.0-alpha
[0.1.0]: https://github.com/iurii-izman/slova/releases/tag/v0.1.0
