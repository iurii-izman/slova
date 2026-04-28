# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive GitHub repository structure with issue templates, PR templates, and workflows
- CI/CD pipeline with GitHub Actions for testing on Windows, macOS, and Linux
- Full README with features, quick start guide, and architecture documentation
- Contributing guidelines and Code of Conduct
- `.editorconfig` for consistent code formatting across editors
- Complete project metadata in Cargo.toml and package.json

### Changed
- Updated Tauri configuration with proper bundle settings for all platforms
- Enhanced .gitignore with comprehensive patterns for all common development files
- Improved frontend package.json with complete metadata and dependencies

### Fixed
- Tauri application identifier updated to proper GitHub-based identifier

## [0.1.0] - 2025-04-28

### Added

#### Phase 1: Persistence Layer ✅ Complete

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

### Architecture

- **Layered architecture:**
  - Types layer (domain models)
  - Database layer (SQLite + repositories)
  - Adapters layer (external services)
  - Core layer (business logic, coming in Phase 2)
  - App layer (Tauri IPC handlers)
  - Frontend layer (Solid.js UI)

- **Security-first design:**
  - API keys in OS keychain
  - Type-safe error handling
  - Input validation
  - No hardcoded secrets

### Known Limitations

- ⚠️ FFmpeg integration not yet implemented
- ⚠️ Groq API client not yet connected
- ⚠️ Job scheduler not running (commands are stubs)
- ⚠️ UI doesn't update in real-time
- ⚠️ Database not yet wired to app state

These are planned for Phase 2–3 development.

[Unreleased]: https://github.com/iurii-izman/slova/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/iurii-izman/slova/releases/tag/v0.1.0
