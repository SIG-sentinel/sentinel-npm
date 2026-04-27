# Workflow: Contributing to Sentinel-NPM After Hardening

## Overview

The `main` branch is now protected with comprehensive security rules. All development MUST follow this workflow.

---

## Required Setup (One-time)

### 1. Configure Git Signing

All commits to the project must be signed. Choose one method:

#### Option A: GPG Signing (Recommended)

```bash
# Generate GPG key (if you don't have one)
gpg --full-generate-key

# Get your key ID
gpg --list-secret-keys --keyid-format LONG

# Configure Git to use your GPG key
git config --global user.signingkey <YOUR_KEY_ID>

# Enable signing globally
git config --global commit.gpgsign true

# If on macOS, configure GPG agent
# (See: https://github.com/keybase/keybase-issues/issues/2798)
```

#### Option B: SSH Signing

```bash
# Generate SSH key
ssh-keygen -t ed25519 -C "your-email@example.com"

# Configure Git to use SSH signing
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ed25519.pub
git config --global commit.gpgsign true
```

### 2. Verify Signing

```bash
# Test a signed commit
git commit --allow-empty -S -m "Test signed commit"

# Verify the commit was signed
git log --show-signature -1
```

---

## Contribution Workflow

### Step 1: Create Feature Branch

```bash
# Always branch from main
git checkout main
git pull origin main

# Create feature branch (use descriptive name)
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number
# or
git checkout -b refactor/module-name
```

### Step 2: Make Changes

```bash
# Edit files as needed
git add .
git commit -S -m "feat: description of your changes"
# or -S flag ensures signing
# Omit -S if commit.gpgsign is already configured globally
```

### Step 3: Verify Locally

Before pushing, ensure code quality:

```bash
# Run tests
cargo test

# Run linter
cargo clippy

# Build
cargo build --release
```

### Step 4: Push to Remote

```bash
git push origin feature/your-feature-name
```

This creates a pull request link in the output.

### Step 5: Create Pull Request

1. Go to GitHub: https://github.com/SIG-sentinel/sentinel-npm
2. Click "New Pull Request"
3. Set:
   - **Base**: `main`
   - **Compare**: your feature branch
4. Add description and submit

### Step 6: Address Review

The code owner (@chrisJSeng) will review:

- Go through the code review comments
- Make requested changes locally:
  ```bash
  git add .
  git commit -S -m "fix: address review comments"
  git push origin feature/your-feature-name
  ```
- New commits automatically update the PR

### Step 7: Wait for CI/CD

**Required Status Checks:**
- ✓ build - Your code must compile
- ✓ test - All 81 tests must pass
- ✓ lint - Clippy must pass

*Note: CI workflows will be created in GitHub Actions*

### Step 8: Merge

Once all conditions are met:
- ✅ At least 1 approval from code owner
- ✅ All status checks passing
- ✅ No merge conflicts

The "Merge" button becomes green. Click to merge.

---

## Branch Protection Rules Explained

### What's Enforced

| Rule | Effect |
|------|--------|
| **No Direct Pushes** | Must use PR, even for admins |
| **Code Owner Review** | @chrisJSeng must approve |
| **Status Checks (Build/Test/Lint)** | All must pass |
| **Signed Commits** | GPG or SSH signature required |
| **No Force Pushes** | Cannot rewrite history after push |
| **No Deletions** | Cannot delete the branch |
| **Stale Review Dismissal** | New commits require new review |

### Why These Rules Exist

1. **Code Quality**: Status checks ensure tests pass
2. **Security**: Commits are verified (signed)
3. **Process**: Code review prevents bugs
4. **Safety**: No accidental deletions or force pushes
5. **Accountability**: Every change is reviewed and traced

---

## Common Tasks

### Rebase Before Merge

If `main` moved ahead of your branch:

```bash
git fetch origin
git rebase origin/main
git push -f origin feature/your-feature-name
# (Force push on your own branch is fine, it resets the review anyway)
```

### Update PR Description

1. Go to the PR on GitHub
2. Click the "..." menu
3. Select "Edit"
4. Update the description
5. Save

### Close PR Without Merging

1. Click "Close pull request" button
2. Optionally delete the branch

### Work on Multiple Features

```bash
# Each feature gets its own branch
git worktree add ../sentinel-feature-2 -b feature/another-feature

# This creates a separate working directory
cd ../sentinel-feature-2
# Now make changes independently
```

---

## Troubleshooting

### ❌ "Your branch has diverged"

```bash
git fetch origin
git rebase origin/main
git push -f origin feature/your-feature-name
```

### ❌ "Commit was not signed"

Make sure GPG/SSH is configured:
```bash
git log --show-signature -1  # Check your last commit
git config commit.gpgsign    # Verify setting
```

### ❌ "Status checks failed"

1. Pull latest `main`: `git pull origin main`
2. Run locally: `cargo test && cargo clippy && cargo build`
3. Fix any issues
4. Commit and push: `git commit -S && git push`

### ❌ "Merge conflict"

```bash
# Update your branch with latest main
git fetch origin
git rebase origin/main

# Resolve conflicts in your editor
# Mark as resolved
git add .
git rebase --continue
git push -f origin feature/your-feature-name
```

### ❌ Can't push to main directly?

This is intentional! Go through PR instead:

```bash
# Create feature branch
git checkout -b feature/my-change

# Make changes
git add .
git commit -S -m "your message"
git push origin feature/my-change

# Then create PR on GitHub
```

---

## Help & Support

- **Questions about workflow?** Ask @chrisJSeng in a PR comment
- **CI/CD issues?** Check GitHub Actions logs (once GitHub Actions workflows are setup)
- **Git issues?** Consult: https://git-scm.com/doc

---

## Compliance Checklist

Before submitting a PR, verify:

- [ ] Branched from `main`
- [ ] All commits are signed (`git log --show-signature`)
- [ ] Code builds locally (`cargo build --release`)
- [ ] All tests pass locally (`cargo test`)
- [ ] Clippy passes locally (`cargo clippy`)
- [ ] PR description is clear
- [ ] No force pushes to `main`
- [ ] No secrets in commits (API keys, passwords, etc.)

---

**Last Updated**: $(date -u +'%Y-%m-%d %H:%M:%S UTC')
**Enforced Branch**: main
**Policy Owner**: @chrisJSeng
