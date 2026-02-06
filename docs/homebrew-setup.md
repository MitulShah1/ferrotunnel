# Homebrew Setup Guide

This guide explains how to publish FerroTunnel to Homebrew.

## Option 1: Homebrew Core (Recommended for Popular Tools)

To get FerroTunnel into the main Homebrew repository:

1. First, publish v1.0.1 release on GitHub
2. Generate the SHA256 checksum:
   ```bash
   curl -sL https://github.com/MitulShah1/ferrotunnel/archive/refs/tags/v1.0.1.tar.gz | shasum -a 256
   ```
3. Update `Formula/ferrotunnel.rb` with the SHA256
4. Submit PR to [homebrew-core](https://github.com/Homebrew/homebrew-core)

**Requirements for homebrew-core:**
- At least 75 stars on GitHub
- Well-maintained project
- Passes all Homebrew audit checks

## Option 2: Personal Tap (Easier, Immediate)

Create your own tap for faster deployment:

### Step 1: Create a Homebrew Tap Repository

```bash
# Create a new repo: homebrew-ferrotunnel
# GitHub naming convention: homebrew-<tap-name>
```

### Step 2: Add the Formula

```bash
git clone https://github.com/MitulShah1/homebrew-ferrotunnel.git
cd homebrew-ferrotunnel
mkdir Formula
cp /path/to/ferrotunnel/Formula/ferrotunnel.rb Formula/
git add Formula/ferrotunnel.rb
git commit -m "Add ferrotunnel formula"
git push
```

### Step 3: Update SHA256 After Release

```bash
# After creating v1.0.1 release, get the SHA256
curl -sL https://github.com/MitulShah1/ferrotunnel/archive/refs/tags/v1.0.1.tar.gz | shasum -a 256

# Update the sha256 line in Formula/ferrotunnel.rb
# Commit and push
```

### Step 4: Test Installation

```bash
# Users can now install with:
brew tap MitulShah1/ferrotunnel
brew install ferrotunnel

# Or in one command:
brew install MitulShah1/ferrotunnel/ferrotunnel
```

## Testing the Formula Locally

Before publishing, test the formula:

```bash
# Audit the formula
brew audit --strict Formula/ferrotunnel.rb

# Test installation locally
brew install --build-from-source Formula/ferrotunnel.rb

# Run tests
brew test ferrotunnel

# Uninstall
brew uninstall ferrotunnel
```

## Formula Template Notes

The formula in `Formula/ferrotunnel.rb`:
- Uses Rust toolchain for building
- Installs from `ferrotunnel-cli` crate
- Includes basic smoke tests
- Supports both stable releases and `HEAD` (latest main)

## After Publishing

Add to README.md:

```markdown
### macOS (Homebrew)

\`\`\`bash
brew tap MitulShah1/ferrotunnel
brew install ferrotunnel
\`\`\`

Or directly:

\`\`\`bash
brew install MitulShah1/ferrotunnel/ferrotunnel
\`\`\`
```

## Updating for New Releases

For each new version:

1. Create GitHub release (e.g., v1.0.2)
2. Get new SHA256:
   ```bash
   curl -sL https://github.com/MitulShah1/ferrotunnel/archive/refs/tags/v1.0.2.tar.gz | shasum -a 256
   ```
3. Update `url` and `sha256` in formula
4. Commit and push to tap repository
5. Users update with: `brew upgrade ferrotunnel`
