# Deployment Guide

This guide covers building, packaging, and distributing VideoTranscriber for all supported platforms.

## Build Requirements

### All Platforms
- Rust 1.70+ (stable)
- Node.js 18+
- pnpm/npm/yarn

### Windows
- Visual Studio Build Tools 2019+ (with C++ workload)
- Windows 10 SDK or later

### macOS
- Xcode 12+
- macOS 10.13+

### Linux
- GCC 9+ or Clang 10+
- libssl-dev, libgtk-3-dev, webkit2gtk-4.0
- For Fedora: `dnf groupinstall "C Development Tools and Libraries"`
- For Ubuntu/Debian: `sudo apt-get install libssl-dev libgtk-3-dev libwebkit2gtk-4.0-dev`

## Building for Development

```bash
# Clone repository
git clone https://github.com/iurii-izman/slova.git
cd slova

# Install frontend dependencies
cd apps/ui
npm install
cd ../..

# Build in development mode
cd src-tauri
cargo build
cargo run --features with_tauri
```

## Building for Release

### Single Platform

```bash
# Navigate to backend
cd src-tauri

# Build release binary (native target)
cargo build --release --features with_tauri
```

Binary location:
- **Windows:** `target/release/slova-tauri.exe`
- **macOS:** `target/release/slova-tauri`
- **Linux:** `target/release/slova-tauri`

### Creating Installers

Tauri supports bundling:

```bash
cd src-tauri

# Windows: Creates MSI and NSIS installers
cargo tauri build --config "{ \"build\": { \"beforeDevCommand\": \"\", \"beforeBuildCommand\": \"cd ../apps/ui && npm run build\" } }"

# macOS: Creates DMG
cargo tauri build --target universal-apple-darwin

# Linux: Creates AppImage and .deb
cargo tauri build
```

Installers will be in `src-tauri/target/release/bundle/`.

## Optimization for Release

### Cargo Release Profile

The workspace uses standard Rust release settings. For even smaller binaries:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

Add this to `src-tauri/Cargo.toml` for ~20% smaller binary at cost of longer compile time.

### Frontend Optimization

Vite already optimizes in production build:

```bash
cd apps/ui
npm run build
```

This creates:
- Minified CSS and JavaScript
- Code splitting
- Tree-shaking
- Asset hashing for caching

## Publishing Releases

### 1. Create GitHub Release

```bash
# Create release tag
git tag v0.2.0
git push origin v0.2.0

# Create release on GitHub with notes
# - Go to https://github.com/iurii-izman/slova/releases
# - Click "Create a new release"
# - Tag version: v0.2.0
# - Add release notes from CHANGELOG.md
# - Upload binaries and installers
```

### 2. Manual Distribution

1. Build for all platforms (see above)
2. Collect binaries/installers:
   - Windows: `.exe` installer
   - macOS: `.dmg` file
   - Linux: `.AppImage` or `.deb`
3. Create checksums:
   ```bash
   sha256sum slova-tauri.exe > slova-tauri.exe.sha256
   sha256sum slova-tauri-*.dmg > slova-tauri.dmg.sha256
   sha256sum slova-tauri*.AppImage > slova-tauri.AppImage.sha256
   ```
4. Upload to release with checksums

### 3. Auto-Update (Future)

When implemented, use Tauri Updater:

```bash
# Generate signature
cargo tauri build -- --sign-updates
```

This creates:
- Private key for signing
- Public key for Tauri config
- Signed release manifest

## Cross-Compilation

### Building for macOS from Linux

Requires osxcross setup (complex). Easier to build on native macOS.

### Building for Linux from Windows

Use WSL2:

```bash
wsl
cd /mnt/c/Dev/slova
cargo build --release
```

### Building for Windows from macOS/Linux

Use MinGW:

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Code Signing (macOS)

### Prerequisites
- Apple Developer Account
- Code signing certificate

### Signing

```bash
# Create signing identity
security import certificate.p12 -k ~/Library/Keychains/login.keychain-db

# Set in tauri.conf.json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Your Signing Identity"
    }
  }
}

# Build with signing
cargo tauri build
```

## Notarization (macOS)

Apple requires notarization for apps distributed outside App Store:

```bash
# After signing, notarize
xcrun altool --notarize-app \
  -f "target/release/bundle/dmg/slova-tauri_0.1.0_x64.dmg" \
  -t osx \
  -u "your-apple-id@example.com" \
  -p "app-specific-password"

# Check status
xcrun altool --notarization-history \
  0 -u "your-apple-id@example.com" \
  -p "app-specific-password"
```

## Performance Optimization

### Binary Size Reduction

1. **Strip symbols:**
   ```bash
   # Cargo.toml
   [profile.release]
   strip = true
   ```

2. **Link-time optimization:**
   ```bash
   [profile.release]
   lto = true
   ```

3. **Single codegen unit:**
   ```bash
   [profile.release]
   codegen-units = 1
   ```

### Startup Time

1. Keep dependencies minimal
2. Avoid heavy initialization in main thread
3. Use lazy_static for expensive computations

## Testing Before Release

### Pre-Release Checklist

- [ ] Rust: `cargo test`
- [ ] TypeScript: `npm run check`, `npm run build`
- [ ] Linting: `cargo clippy`, `cargo fmt --check`
- [ ] No hardcoded secrets or API keys
- [ ] No debug prints or logging at info level
- [ ] Keyboard shortcuts work
- [ ] Drag & drop works on all platforms
- [ ] Settings persist across restarts
- [ ] API key storage works (test on target platform)
- [ ] Database migrations work fresh
- [ ] Error messages are user-friendly

### Platform-Specific Testing

**Windows:**
- Test on Windows 10, 11
- Test installer/uninstaller
- Test Windows Defender interactions

**macOS:**
- Test on Intel and Apple Silicon
- Test code signing/notarization
- Test Gatekeeper interactions
- Test auto-update (once implemented)

**Linux:**
- Test on Ubuntu 20.04, 22.04
- Test on Fedora, Debian variants
- Test AppImage permissions
- Test .deb installation

## Deployment Architecture

```
Release Branch (stable)
       ↓
  Tagged Release
       ↓
  Build Artifacts (CI/CD)
       ↓
  Upload to GitHub Releases
       ↓
  Publish on website/store
```

## Rollback Procedure

If a release has critical bugs:

1. Delete or mark release as "pre-release"
2. Don't update auto-updater manifest
3. Create patch release (e.g., 0.1.1) with fixes
4. Tag and re-release

## Continuous Deployment (Future)

When ready to automate:

1. Set up GitHub Actions release workflow
2. Auto-build on tag push
3. Auto-publish artifacts
4. Auto-update Tauri updater manifest
5. Notify users of update availability

See `.github/workflows/` for CI setup.

## Troubleshooting

### Build Fails on Windows

```bash
# Ensure Visual Studio Build Tools installed
# Or use WSL2 for Linux build

# Clean rebuild
cargo clean
cargo build --release
```

### macOS Code Signing Issues

```bash
# List signing identities
security find-identity -v -p codesigning ~/Library/Keychains/login.keychain-db

# If signing fails, check certificate expiry
```

### Linux Symbol Not Found

```bash
# Ensure all dependencies installed
ldd target/release/slova-tauri

# Install missing libraries
sudo ldconfig
```

## Resources

- [Tauri Bundler Documentation](https://tauri.app/v1/guides/building/)
- [Rust Book - Release Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [macOS Code Signing Guide](https://developer.apple.com/support/code-signing/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)

## Support

For deployment issues:
1. Check [GitHub Issues](https://github.com/iurii-izman/slova/issues)
2. Open new issue with build logs
3. Check Tauri [Discord community](https://tauri.app/en/community/)
