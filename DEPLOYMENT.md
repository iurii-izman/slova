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

### Option 1: Automated CI/CD with GitHub Actions (Recommended)

See [PACKAGING.md](./PACKAGING.md) for complete instructions.

**Quick start:**
```bash
# Create version tag
git tag v0.2.0
git push origin v0.2.0
```

GitHub Actions automatically:
1. Builds Windows NSIS + MSI installers
2. Runs all tests and linting
3. Creates GitHub Release with artifacts
4. Generates release notes from commits

See `.github/workflows/release-windows.yml` for the workflow definition.

### Option 2: Local Build

**Single Platform:**

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

**Creating Installers:**

Tauri supports bundling:

```bash
cd src-tauri

# Windows: Creates MSI and NSIS installers with bundled FFmpeg
cargo tauri build

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

### 1. Automated Release (GitHub Actions)

**Best for CI/CD:**

```bash
# Update versions in code
# Edit Cargo.toml, package.json, tauri.conf.json with new version

# Create release tag
git tag v0.2.0
git push origin v0.2.0

# GitHub Actions automatically:
# - Builds all installers
# - Runs tests and linting
# - Creates release on GitHub
# - Uploads artifacts
```

Monitor progress at: **GitHub Actions** tab

### 2. Manual Distribution

If building locally:

1. Build for all platforms (see [Option 2](#option-2-local-build) above)
2. Collect binaries/installers:
   - Windows: `.exe` and `.msi` installers
   - macOS: `.dmg` file
   - Linux: `.AppImage` or `.deb`
3. Create checksums:
   ```bash
   sha256sum VideoTranscriber_0.2.0_installer_x64.exe > sha256sums.txt
   sha256sum VideoTranscriber_0.2.0_x64.msi >> sha256sums.txt
   ```
4. Go to [GitHub Releases](https://github.com/iurii-izman/slova/releases)
5. Click "Create a new release"
6. Tag version: `v0.2.0`
7. Add release notes from [CHANGELOG.md](./CHANGELOG.md)
8. Upload binaries, installers, and checksums
9. Publish

### 3. Auto-Update (When Ready)

See [PACKAGING.md — Auto-Update Configuration](./PACKAGING.md#auto-update-configuration).

When implemented, use Tauri Updater:

```bash
# Generate signature key (one time)
cargo tauri signer generate --ci

# Build and sign
cargo tauri build -- --sign-updates-key YOUR_PRIVATE_KEY
```

This creates:
- Signed updates manifest
- Public key for Tauri config
- Releases are auto-delivered to users

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

## Code Signing

### Windows Code Signing (Optional)

⚠️ **See [PACKAGING.md — Code Signing & Security](./PACKAGING.md#code-signing--security) for detailed instructions.**

To avoid "Unknown publisher" warning on installer:

1. Purchase Authenticode certificate from trusted CA (Sectigo, DigiCert, etc.)
2. Store `.pfx` file securely — **never commit to repo**
3. Set GitHub Actions secrets (if using CI):
   - `CERTIFICATE_PFX_BASE64` — base64-encoded .pfx
   - `CERTIFICATE_PASSWORD` — private key password
4. Uncomment signing step in `.github/workflows/release-windows.yml`

**Default (no signing):** Windows shows warning, but app installs fine. Recommended for open-source projects.

### macOS Code Signing (When Applicable)

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

See [PACKAGING.md — Release Checklist](./PACKAGING.md#release-checklist) for comprehensive checklist.

**Quick checks:**
- [ ] Update version in `Cargo.toml`, `package.json`, `tauri.conf.json`
- [ ] Update `CHANGELOG.md` with release notes
- [ ] Rust: `cargo test --lib`
- [ ] TypeScript: `npm run check` and `npm run build`
- [ ] Linting: `cargo clippy`, `cargo fmt -- --check`
- [ ] No hardcoded secrets or API keys
- [ ] Build locally: `cargo tauri build`
- [ ] Test installer on Windows 10, 11
- [ ] Test uninstall process
- [ ] Verify API key storage in Credential Manager

### Platform-Specific Testing

**Windows:**
- Test NSIS installer with default settings
- Test MSI installer with default settings
- Test installer/uninstaller
- Test Windows Defender interactions
- Verify FFmpeg/ffprobe extraction
- Check Start Menu shortcuts

**macOS (Future):**
- Test on Intel and Apple Silicon
- Test code signing/notarization
- Test Gatekeeper interactions
- Test auto-update (once implemented)

**Linux (Future):**
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
