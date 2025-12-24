# Distribution Guide for Sentinel

This guide explains how to distribute Sentinel via curl install script and Homebrew.

---

## 📦 Distribution Methods

### 1. Curl Install Script (Cross-platform)
### 2. Homebrew Formula (macOS/Linux)
### 3. Pre-built Binaries (GitHub Releases)
### 4. Build from Source

---

## 🚀 Quick Start for Users

### Method 1: Curl Install (Recommended)

```bash
# Install latest release
curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash

# Build from source
curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash -s -- --source

# Custom install directory
INSTALL_DIR=$HOME/.local/bin curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash
```

### Method 2: Homebrew (macOS/Linux)

```bash
# From Homebrew tap (after setting up tap)
brew install engramai-io/tap/sentinel

# From local formula
brew install --build-from-source ./Formula/sentinel.rb

# Upgrade
brew upgrade sentinel
```

---

## 🛠 Setting Up Distribution (For Maintainers)

### Step 1: Create GitHub Releases with Binaries

#### Option A: Manual Release

1. **Build binaries for all platforms:**

```bash
# macOS x86_64
cargo build --release --target x86_64-apple-darwin

# macOS ARM64 (M1/M2)
cargo build --release --target aarch64-apple-darwin

# Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# Windows x86_64
cargo build --release --target x86_64-pc-windows-msvc
```

2. **Create GitHub Release:**
   - Go to: https://github.com/EngramAI-io/Core/releases/new
   - Tag: `v0.2.0` (semantic versioning)
   - Title: `Sentinel v0.2.0 - Security & UI Updates`
   - Upload binaries with naming convention:
     - `sentinel-linux-x86_64`
     - `sentinel-linux-aarch64`
     - `sentinel-darwin-x86_64`
     - `sentinel-darwin-aarch64`
     - `sentinel-windows-x86_64.exe`

#### Option B: Automated Release with GitHub Actions

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            binary_name: sentinel-linux-x86_64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            binary_name: sentinel-linux-aarch64
          - os: macos-latest
            target: x86_64-apple-darwin
            binary_name: sentinel-darwin-x86_64
          - os: macos-latest
            target: aarch64-apple-darwin
            binary_name: sentinel-darwin-aarch64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            binary_name: sentinel-windows-x86_64.exe
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
      
      - name: Setup Node
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Build Frontend
        run: |
          cd frontend/my-react-flow-app
          npm install
          npm run build
      
      - name: Build Binary
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Prepare Binary
        run: |
          if [ "${{ matrix.os }}" = "windows-latest" ]; then
            cp target/${{ matrix.target }}/release/sentinel.exe ${{ matrix.binary_name }}
          else
            cp target/${{ matrix.target }}/release/sentinel ${{ matrix.binary_name }}
          fi
        shell: bash
      
      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ github.event.release.upload_url }}
          asset_path: ./${{ matrix.binary_name }}
          asset_name: ${{ matrix.binary_name }}
          asset_content_type: application/octet-stream
```

### Step 2: Update SHA256 in Homebrew Formula

After creating a GitHub release:

```bash
# Download the source tarball
curl -L https://github.com/EngramAI-io/Core/archive/refs/tags/v0.2.0.tar.gz -o sentinel.tar.gz

# Calculate SHA256
sha256sum sentinel.tar.gz  # Linux
# or
shasum -a 256 sentinel.tar.gz  # macOS

# Update Formula/sentinel.rb with the SHA256
```

### Step 3: Create Homebrew Tap (One-time setup)

```bash
# Create a new GitHub repository: homebrew-tap
# Add the formula to it

# Users will then install with:
brew tap engramai-io/tap
brew install sentinel
```

**Repository structure for `homebrew-tap`:**
```
homebrew-tap/
├── Formula/
│   └── sentinel.rb
└── README.md
```

---

## 📋 Release Checklist

### Before Release

- [ ] Update version in `Cargo.toml`
- [ ] Update version in `Formula/sentinel.rb`
- [ ] Update CHANGELOG.md
- [ ] Run tests: `cargo test --all`
- [ ] Build frontend: `cd frontend && npm run build`
- [ ] Test install script locally
- [ ] Test Homebrew formula locally

### Creating Release

- [ ] Tag release: `git tag -a v0.2.0 -m "Release v0.2.0"`
- [ ] Push tag: `git push origin v0.2.0`
- [ ] Wait for GitHub Actions to build binaries (if automated)
- [ ] Create GitHub Release with binaries
- [ ] Update SHA256 in Homebrew formula
- [ ] Test installation methods:
  - [ ] `curl | bash` install script
  - [ ] Homebrew install
  - [ ] Direct binary download

### After Release

- [ ] Update README badges with latest version
- [ ] Announce on social media / blog
- [ ] Update documentation if needed
- [ ] Monitor for installation issues

---

## 🧪 Testing Installation Methods

### Test Curl Install

```bash
# Test in clean environment (Docker)
docker run -it --rm ubuntu:latest bash
curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash

# Test on macOS
curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash

# Test build from source
curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash -s -- --source
```

### Test Homebrew Install

```bash
# Test local formula
brew install --build-from-source ./Formula/sentinel.rb
brew test sentinel
brew uninstall sentinel

# Test from tap (after publishing)
brew install engramai-io/tap/sentinel
sentinel --version
```

---

## 🔧 Customization Options

### Custom Install Directory

Users can specify a custom installation directory:

```bash
INSTALL_DIR=$HOME/.local/bin curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash
```

### Offline Installation

For air-gapped environments:

```bash
# 1. Download install script
curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh -o install-sentinel.sh

# 2. Download binary manually
curl -L https://github.com/EngramAI-io/Core/releases/download/v0.2.0/sentinel-linux-x86_64 -o sentinel

# 3. Install manually
chmod +x sentinel
sudo mv sentinel /usr/local/bin/
```

---

## 📊 Distribution Analytics

Track installation metrics:

- GitHub Release download counts
- Homebrew analytics (if opted in)
- curl script usage (via proxy/CDN logs)

---

## 🐛 Troubleshooting

### Install Script Issues

**Problem:** `curl: command not found`
```bash
# Use wget instead
wget -qO- https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash
```

**Problem:** Permission denied
```bash
# Install to user directory
INSTALL_DIR=$HOME/.local/bin curl -sSL https://... | bash
```

**Problem:** Binary not found for platform
```bash
# Force build from source
curl -sSL https://... | bash -s -- --source
```

### Homebrew Issues

**Problem:** Formula fails to build
```bash
# Check dependencies
brew doctor

# Install with verbose output
brew install --build-from-source --verbose sentinel
```

**Problem:** Old version installed
```bash
# Update and upgrade
brew update
brew upgrade sentinel
```

---

## 📝 Best Practices

1. **Version all releases**: Use semantic versioning (v0.2.0, v0.2.1, etc.)
2. **Sign binaries**: Use GPG to sign release binaries
3. **Checksum verification**: Provide SHA256 checksums for all downloads
4. **Test on clean systems**: Always test in fresh Docker containers
5. **Document breaking changes**: Keep detailed CHANGELOG.md
6. **Support LTS versions**: Maintain backwards compatibility
7. **Security patches**: Release quickly for security fixes

---

## 🔐 Security Considerations

### Binary Signing

```bash
# Sign macOS binary
codesign --force --verify --verbose --sign "Developer ID" sentinel

# Sign with GPG
gpg --armor --detach-sign sentinel-linux-x86_64
```

### Checksum File

Create `SHA256SUMS` for releases:

```bash
sha256sum sentinel-* > SHA256SUMS
gpg --clearsign SHA256SUMS
```

Users verify:

```bash
sha256sum -c SHA256SUMS
gpg --verify SHA256SUMS.asc
```

---

## 📚 Additional Resources

- [Rust Cross Compilation](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [GitHub Releases API](https://docs.github.com/en/rest/releases)
- [Semantic Versioning](https://semver.org/)

---

## 🎯 Next Steps

1. Set up GitHub Actions for automated releases
2. Create homebrew-tap repository
3. Test install methods on all platforms
4. Document installation in main README
5. Create demo video showing installation
