# GitHub Repository Configuration Script
# Usage: .\setup-github.ps1

Write-Host "GitHub Repository Configuration Script" -ForegroundColor Cyan
Write-Host "Repository: iurii-izman/slova" -ForegroundColor Cyan
Write-Host ""

# Step 1: Add topics
Write-Host "[1/6] Adding repository topics..." -ForegroundColor Cyan
$topics = @("video", "transcription", "whisper-api", "groq", "tauri", "rust")
foreach ($topic in $topics) {
    gh repo edit iurii-izman/slova --add-topic $topic 2>$null
    Write-Host "  Added topic: $topic" -ForegroundColor Green
}
Write-Host ""

# Step 2: Enable discussions
Write-Host "[2/6] Enabling discussions..." -ForegroundColor Cyan
gh repo edit iurii-izman/slova --enable-discussions 2>$null
Write-Host "  Discussions enabled" -ForegroundColor Green
Write-Host ""

# Step 3: Configure branch protection
Write-Host "[3/6] Configuring branch protection..." -ForegroundColor Cyan
gh api repos/iurii-izman/slova/branches/main/protection -X PUT `
  -f required_status_checks='{"strict":false,"contexts":["tests"]}' `
  -f required_pull_request_reviews='{"dismiss_stale_reviews":true,"require_code_owner_reviews":false,"required_approving_review_count":1}' `
  -f dismiss_stale_reviews=true `
  -f require_code_owner_reviews=false `
  -f allow_force_pushes=true `
  -f allow_deletions=false 2>$null
Write-Host "  Branch protection configured" -ForegroundColor Green
Write-Host ""

# Step 4: Create labels
Write-Host "[4/6] Creating issue labels..." -ForegroundColor Cyan
$labels = @(
    @{name="bug"; color="d73a4a"},
    @{name="enhancement"; color="a2eeef"},
    @{name="documentation"; color="0075ca"},
    @{name="good first issue"; color="7057ff"},
    @{name="help wanted"; color="008672"},
    @{name="phase-1"; color="cccccc"},
    @{name="phase-2"; color="cccccc"},
    @{name="phase-3"; color="cccccc"},
    @{name="security"; color="ee0701"}
)

foreach ($label in $labels) {
    gh label create $label.name --color $label.color -R iurii-izman/slova 2>$null
    Write-Host "  Created label: $($label.name)" -ForegroundColor Green
}
Write-Host ""

# Step 5: Create milestones
Write-Host "[5/6] Creating milestones..." -ForegroundColor Cyan
$milestones = @(
    @{title="Phase 2: Queue Scheduler"},
    @{title="Phase 3: UI and Export"},
    @{title="Phase 4: Polish"},
    @{title="Post-MVP"}
)

foreach ($milestone in $milestones) {
    gh api repos/iurii-izman/slova/milestones -X POST -f title=$milestone.title 2>$null
    Write-Host "  Created milestone: $($milestone.title)" -ForegroundColor Green
}
Write-Host ""

# Step 6: Summary
Write-Host "[6/6] Configuration complete!" -ForegroundColor Cyan
Write-Host ""
Write-Host "Done! Your repository is configured:" -ForegroundColor Green
Write-Host "  - 6 topics added" -ForegroundColor Green
Write-Host "  - Discussions enabled" -ForegroundColor Green
Write-Host "  - Branch protection (soft)" -ForegroundColor Green
Write-Host "  - 9 labels created" -ForegroundColor Green
Write-Host "  - 4 milestones created" -ForegroundColor Green
Write-Host ""
Write-Host "Repository: https://github.com/iurii-izman/slova" -ForegroundColor Cyan
