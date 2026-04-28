# Contributing to VideoTranscriber

Thank you for your interest in contributing! This document provides guidelines for contributing to the VideoTranscriber project.

## Code of Conduct

Be respectful, inclusive, and professional in all interactions.

## Before You Start

1. **Read the Architecture Documents:**
   - `transcriber-spec.md` — technical specification and requirements
   - `transcriber-architecture-analysis.md` — detailed architecture and design decisions
   - `transcriber-autopilot-development-plan.md` — development blocks and implementation strategy

2. **Understand the Stack:**
   - **Desktop:** Tauri 2
   - **Backend:** Rust + Tokio
   - **Frontend:** Solid.js + TypeScript + Vite
   - **Database:** SQLite
   - **STT:** Groq Whisper Large v3 Turbo
   - **Secrets:** OS keychain via `keyring` crate

## Development Setup

### Prerequisites

- Rust 1.70+ (stable)
- Node.js 18+
- pnpm/npm/yarn
- FFmpeg and FFprobe binaries in PATH (for media processing)

### Setup Steps

```bash
# Clone the repository
git clone https://github.com/iurii-izman/slova.git
cd slova

# Install backend dependencies (automatic with cargo)
# Install frontend dependencies
cd apps/ui
pnpm install
cd ../..
```

### Running in Development

**Terminal 1 — UI Dev Server:**
```bash
cd apps/ui
pnpm dev
```

**Terminal 2 — Tauri Backend:**
```bash
cd src-tauri
cargo run --features with_tauri
```

## Code Style & Standards

### Rust

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Write documentation comments for public APIs
- Use typed errors — no stringly-typed errors

**Checks before committing:**
```bash
cd src-tauri
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

### TypeScript/JavaScript

- Use TypeScript for all new code
- Follow ESLint configuration
- Use `const` by default, `let` only when necessary
- Prefer functional components in Solid.js

**Checks before committing:**
```bash
cd apps/ui
pnpm run build
pnpm run check
```

## Security & Best Practices

### Do's ✅

- Store API keys in OS keychain, never in code or environment files
- Use typed results for error handling
- Write unit tests for new functionality
- Document complex business logic
- Validate user input on both UI and backend
- Use safe process APIs for external tools (FFmpeg, FFprobe)

### Don'ts ❌

- Never hardcode secrets, API keys, or tokens
- Don't use shell command concatenation for FFmpeg/FFprobe
- Don't add plaintext credential files to the repository
- Don't make real API requests in tests without explicit user consent
- Don't skip error handling

## Testing

### Unit Tests

```bash
cd src-tauri
cargo test
```

### Type Checking

```bash
# TypeScript
cd apps/ui
pnpm run check

# Rust
cd src-tauri
cargo check
```

### Integration Testing

UI integration tests via Solid.js testing utilities (to be added).

## Commit Guidelines

- Write clear, descriptive commit messages
- Use the present tense: "Add feature" instead of "Added feature"
- Reference issues when applicable: "Fix #123"
- Keep commits atomic and focused

**Example:**
```
Add exponential backoff for Groq API rate limiting

- Implement backoff strategy with max 5 retries
- Add exponential delay: 1s, 2s, 4s, 8s, 16s
- Update JobState to track retry attempts
- Add tests for backoff logic

Closes #45
```

## Pull Request Process

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes and commit:**
   - Run all checks (`cargo fmt`, `cargo clippy`, `cargo test`)
   - Ensure no hardcoded secrets
   - Update documentation if needed

3. **Push and create a PR:**
   ```bash
   git push origin feature/your-feature-name
   ```

4. **PR Description should include:**
   - What problem does this solve?
   - How does it solve it?
   - Any architecture changes or trade-offs?
   - Testing performed

## Documentation Updates

If your change affects:
- **Architecture:** Update `transcriber-architecture-analysis.md`
- **Specification:** Update `transcriber-spec.md`
- **API/Commands:** Update relevant doc comments and README
- **Development:** Update `transcriber-autopilot-development-plan.md` if adding new development blocks

## Reporting Bugs

Use GitHub Issues with:
- Clear title and description
- Steps to reproduce
- Expected vs. actual behavior
- System info (OS, Rust version, etc.)
- Logs or screenshots if applicable

## Feature Requests

Use GitHub Discussions or Issues with:
- Clear description of the feature
- Why it's needed
- Proposed implementation (if any)
- Impact on architecture or performance

## Architecture Decisions

For significant architectural changes:
1. Create an issue or discussion first
2. Describe the proposal and rationale
3. Address potential impacts on existing code
4. Get feedback before implementation

## Questions?

- Check existing issues and discussions
- Read the architecture documents
- Open a discussion on GitHub

Thank you for contributing to VideoTranscriber! 🚀
