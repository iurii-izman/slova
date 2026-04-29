# VideoTranscriber — Release Instructions

Quick reference for releasing a new version of VideoTranscriber on Windows.

---

## 🚀 Pre-Release (Day Before)

### 1. Update Version Numbers

Update all version references to new version (e.g., `0.2.0`):

**File: `Cargo.toml`** (root workspace)
```toml
[workspace.package]
version = "0.2.0"
```

**File: `apps/ui/package.json`**
```json
{
  "version": "0.2.0"
}
```

**File: `src-tauri/tauri.conf.json`**
```json
{
  "version": "0.2.0"
}
```

### 2. Update CHANGELOG.md

Add new section at top:

```markdown
# VideoTranscriber v0.2.0 — Release Name (if any)

**Release Date:** YYYY-MM-DD  
**Status:** Stable

## What's New

- Feature 1
- Feature 2
- Bug fix 1

## Performance

[if applicable]

## Known Issues

[if any]

## Credits

[contributors, thanks, etc.]
```

### 3. Run Pre-Release Tests

```powershell
cd src-tauri

# Format check
cargo fmt -- --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --lib

# Frontend tests
cd ../apps/ui
npm run check
npm run build
cd ../..
```

### 4. Commit Changes

```bash
git add Cargo.toml apps/ui/package.json src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"
git push origin develop  # or main if that's your default
```

---

## 📦 Release (Tag & Build)

### 5. Create Version Tag

```bash
# Create annotated tag
git tag -a v0.2.0 -m "VideoTranscriber v0.2.0 - Release description"

# Push tag to GitHub
git push origin v0.2.0
```

**This automatically triggers GitHub Actions!**

### 6. Monitor GitHub Actions

Go to **Actions** tab → **Release — Windows**

- ✅ Should see workflow running
- ✅ Takes ~15-20 minutes to complete
- ⚠️ If it fails, check error logs and fix (likely FFmpeg download issue)

**Wait for green checkmark ✅**

---

## ✅ Post-Release (Verification)

### 7. Verify GitHub Release

Go to **Releases** tab:

- ✅ Should see new release `v0.2.0`
- ✅ Should have `.exe` (NSIS) and `.msi` installers
- ✅ Should have auto-generated release notes

**Optional:** Edit release notes if auto-generated notes are wrong.

### 8. Test Installers

**Download and test on Windows 10/11:**

```powershell
# NSIS installer
VideoTranscriber_0.2.0_installer_x64.exe

# Test:
# 1. Run installer
# 2. Follow prompts
# 3. Launch app
# 4. Test basic functionality:
#    - Drag & drop a file
#    - Check Settings
#    - Check logs
# 5. Uninstall via "Add or remove programs"
```

### 9. Announce Release

- [ ] Post to **GitHub Discussions** (optional)
- [ ] Update website/docs if applicable
- [ ] Announce on social media / newsletters (if applicable)

---

## 🔄 If Release Fails

### Workflow Failed in GitHub Actions

1. Check error message in **Actions** tab
2. Common issues:
   - FFmpeg download failed → Check internet, retry tag push
   - Clippy failed → Fix code, commit, retag
   - Tests failed → Fix code, commit, retag

**To retry:**
```bash
# Delete tag locally and on GitHub
git tag -d v0.2.0
git push origin :v0.2.0

# Fix the problem
git commit --amend
git push origin main

# Create new tag
git tag -a v0.2.0 -m "VideoTranscriber v0.2.0"
git push origin v0.2.0
```

### Installer Won't Install

1. Check Windows Defender isn't blocking it
2. Run with admin rights: Right-click → "Run as administrator"
3. If SmartScreen blocks: Click "More info" → "Run anyway"

---

## 📋 Quick Checklist

```
Pre-Release:
- [ ] Update version in Cargo.toml, package.json, tauri.conf.json
- [ ] Update CHANGELOG.md
- [ ] Run cargo fmt, clippy, test
- [ ] Run npm check, build
- [ ] Commit to main/develop
- [ ] Push to GitHub

Release:
- [ ] Create git tag: git tag -a v0.2.0 -m "..."
- [ ] Push tag: git push origin v0.2.0
- [ ] Wait for GitHub Actions to finish (~15-20 min)
- [ ] Verify release artifacts appear

Post-Release:
- [ ] Download and test .exe installer
- [ ] Download and test .msi installer
- [ ] Verify Start Menu shortcuts work
- [ ] Test uninstall
- [ ] Announce release (optional)
```

---

## 🔐 Security Reminders

- ❌ Never commit API keys, certificates, or passwords
- ✅ Use GitHub Actions secrets for sensitive data
- ✅ Keep code signing certificate safe
- ✅ Update FFmpeg when security patches available

---

## 📖 Additional Resources

- **Full packaging guide:** [PACKAGING.md](./PACKAGING.md)
- **Detailed deployment:** [DEPLOYMENT.md](./DEPLOYMENT.md)
- **Architecture reference:** [transcriber-architecture-analysis.md](./transcriber-architecture-analysis.md)
- **GitHub Actions workflow:** [.github/workflows/release-windows.yml](./.github/workflows/release-windows.yml)

---

## ❓ FAQ

### Q: How long does the build take?
**A:** ~15-20 minutes on GitHub Actions (includes Rust compilation, frontend build, and bundling).

### Q: Can I build locally instead of using GitHub Actions?
**A:** Yes, see [PACKAGING.md — Bundling for Release](./PACKAGING.md#bundling-for-release).

### Q: Do I need a code signing certificate?
**A:** No, optional. Without it, Windows shows "Unknown publisher" warning. Users can still install by clicking "More info" → "Run anyway".

### Q: Why are both NSIS and MSI generated?
**A:** Different Windows user preferences. NSIS is simpler; MSI integrates better with enterprise deployment tools.

### Q: How do I update FFmpeg version?
**A:** Edit `src-tauri/tauri.conf.json` under `bundle.externalBin`, update URL and SHA256 hash. See [PACKAGING.md — Sidecar Binaries](./PACKAGING.md#sidecar-binaries).

### Q: What if GitHub Actions fails due to network issues?
**A:** Retry by pushing the tag again. GitHub Actions will rebuild.

---

**🎉 Congrats on the new release!**
