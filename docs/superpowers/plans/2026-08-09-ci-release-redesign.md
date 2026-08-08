# CI Release Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current CI workflow with a full quality gate (fmt, clippy, test) and add a release workflow that builds pre-compiled binaries for 4 platforms and publishes RustDoc to GitHub Pages on tag pushes.

**Architecture:** Two GitHub Actions workflows — `ci.yml` for parallel quality gate on push/PRs, and `release.yml` for tag-triggered binary builds + GitHub Release + GitHub Pages doc publishing. Matrix builds for 4 platform targets with native compilation.

**Tech Stack:** GitHub Actions (v4 actions), Rust stable, `dtolnay/rust-toolchain`, `softprops/action-gh-release@v2`, `peaceiris/actions-gh-pages@v4`

## Global Constraints

- All GitHub Actions use latest major versions (`@v4` or `@v2` as specified)
- Workflow artifact retention: 1 day (`retention-days: 1`)
- Version must match tag name (strip leading `v`) vs `Cargo.toml` — mismatch fails early
- Platform-native archives: `.tar.gz` for Unix, `.zip` for Windows
- Each archive includes: binary + `README.md` + `LICENSE` + `.sha256` checksum
- RustDoc deploys to `gh-pages` branch
- MSVC only for Windows (no GNU target)

---

### Task 1: Rewrite CI Workflow with Full Quality Gate

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: Three parallel jobs (`fmt`, `clippy`, `test`) that run on push to master and PRs using `ubuntu-latest` + stable Rust

This task replaces the current CI workflow's test matrix (stable/beta/nightly across OSes) with a clean parallel quality gate. The current workflow uses `actions-rs/toolchain@v1` — replace with `dtolnay/rust-toolchain@stable`.

- [ ] **Step 1: Rewrite ci.yml**

Replace the entire `.github/workflows/ci.yml` with:

```yaml
name: ci
on:
  pull_request:
  push:
    branches:
      - master

jobs:
  fmt:
    name: rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  clippy:
    name: clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    name: test
    runs-on: ubuntu-latest
    env:
      RUST_BACKTRACE: 1
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --verbose
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: rewrite with parallel quality gate (fmt, clippy, test)"
```

**Deliverable:** CI workflow runs fmt, clippy, and test in parallel on `ubuntu-latest` with stable Rust on every push to master and PR.

---

### Task 2: Create Release Workflow Skeleton with Quality Gate

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `pore-bin/Cargo.toml` (verify binary name)

**Interfaces:**
- Consumes: `ci.yml` quality gate jobs (same fmt/clippy/test logic)
- Produces: `quality` job that gates subsequent build/release jobs

This task creates the release workflow with trigger logic and the quality gate. The binary build and release jobs come in later tasks.

- [ ] **Step 1: Create release.yml with triggers and quality gate**

```yaml
name: release
on:
  push:
    tags:
      - "v*"
  workflow_dispatch:

permissions:
  contents: write
  pages: write
  id-token: write

jobs:
  quality:
    name: quality-gate
    runs-on: ubuntu-latest
    env:
      RUST_BACKTRACE: 1
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Check formatting
        run: cargo fmt --all -- --check
      - name: Run clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Run tests
        run: cargo test --workspace --verbose

  # Version check — fails if tag version doesn't match Cargo.toml
  version-check:
    name: version-check
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/')
    needs: quality
    steps:
      - uses: actions/checkout@v4
      - name: Extract version from tag
        id: tag
        run: echo "version=${GITHUB_REF#refs/tags/v}" >> "$GITHUB_OUTPUT"
      - name: Read Cargo.toml version
        id: cargo
        run: |
          VERSION=$(grep -m1 '^version = ' pore-bin/Cargo.toml | sed 's/version = "\(.*\)"/\1/')
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
      - name: Verify versions match
        run: |
          echo "Tag version: ${{ steps.tag.outputs.version }}"
          echo "Cargo version: ${{ steps.cargo.outputs.version }}"
          if [ "${{ steps.tag.outputs.version }}" != "${{ steps.cargo.outputs.version }}" ]; then
            echo "ERROR: Tag version does not match Cargo.toml version"
            exit 1
          fi
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow skeleton with quality gate and version check"
```

**Deliverable:** Release workflow triggers on `v*` tags and manual dispatch. Runs quality gate (fmt + clippy + test) and verifies tag version matches `Cargo.toml`.

---

### Task 3: Add Binary Build Matrix to Release Workflow

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `quality` job (must pass first)
- Produces: 4 build artifacts uploaded via `actions/upload-artifact@v4` with `retention-days: 1`

Add the `build` matrix job with 4 target triples. Each builds natively, packages the binary + README + LICENSE, generates a SHA256 checksum, and uploads the artifact.

- [ ] **Step 1: Add build matrix to release.yml**

Append after the `version-check` job:

```yaml
  build:
    name: build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    needs: quality
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            archive_ext: tar.gz
          - target: x86_64-apple-darwin
            os: macos-latest
            archive_ext: tar.gz
          - target: aarch64-apple-darwin
            os: macos-latest
            archive_ext: tar.gz
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            archive_ext: zip
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }} --bin pore

      - name: Set version variable
        id: version
        shell: bash
        run: |
          if [ "${{ github.ref_type }}" = "tag" ]; then
            VERSION="${{ github.ref_name }}"
          else
            CARGO_VERSION=$(grep -m1 '^version = ' pore-bin/Cargo.toml | sed 's/version = "\(.*\)"/\1/')
            VERSION="${CARGO_VERSION}-${GITHUB_SHA:0:7}"
          fi
          # Strip leading 'v'
          echo "version=${VERSION#v}" >> "$GITHUB_OUTPUT"

      - name: Package artifacts
        shell: bash
        run: |
          mkdir -p release-artifacts
          BIN_DIR="target/${{ matrix.target }}/release"

          if [ "${{ matrix.os }}" = "windows-latest" ]; then
            cp "$BIN_DIR/pore.exe" release-artifacts/
          else
            cp "$BIN_DIR/pore" release-artifacts/
          fi
          cp README.md LICENSE release-artifacts/

          ARTIFACT_NAME="pore-${{ matrix.target }}"

          if [ "${{ matrix.archive_ext }}" = "zip" ]; then
            cd release-artifacts
            7z a "../${ARTIFACT_NAME}.zip" *
            cd ..
            certutil -hashfile "${ARTIFACT_NAME}.zip" SHA256 | findstr -v "hash" > "${ARTIFACT_NAME}.zip.sha256"
          else
            tar -czf "${ARTIFACT_NAME}.tar.gz" -C release-artifacts .
            sha256sum "${ARTIFACT_NAME}.tar.gz" > "${ARTIFACT_NAME}.tar.gz.sha256"
          fi

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: pore-${{ matrix.target }}
          path: |
            pore-${{ matrix.target }}.${{ matrix.archive_ext }}
            pore-${{ matrix.target }}.${{ matrix.archive_ext }}.sha256
          retention-days: 1
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add build matrix for 4 platform targets in release workflow"
```

**Deliverable:** Release workflow builds 4 platform binaries natively, packages with README+LICENSE, generates SHA256 checksums, uploads as workflow artifacts.

---

### Task 4: Add Release Creation Job

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: all `build` job artifacts via `actions/download-artifact@v4`
- Produces: GitHub Release with uploaded assets (only on tag push)

Add the `release` job that downloads build artifacts and creates a GitHub Release. This only runs on `v*` tag pushes.

- [ ] **Step 1: Add release job to release.yml**

Append after the `build` job:

```yaml
  release:
    name: create-github-release
    runs-on: ubuntu-latest
    needs: build
    if: startsWith(github.ref, 'refs/tags/')
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: release-assets

      - name: Generate release notes
        id: notes
        run: |
          PREV_TAG=$(git tag --sort=-version:refname | grep -v "${{ github.ref_name }}" | head -1 || true)
          if [ -n "$PREV_TAG" ]; then
            NOTES=$(git log "$PREV_TAG"..HEAD --oneline --pretty=format:"%s" || echo "Release ${{ github.ref_name }}")
          else
            NOTES="Initial release"
          fi
          echo "notes<<EOF" >> "$GITHUB_OUTPUT"
          echo "$NOTES" >> "$GITHUB_OUTPUT"
          echo "EOF" >> "$GITHUB_OUTPUT"

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: release-assets/**/*
          generate_release_notes: false
          body: ${{ steps.notes.outputs.notes }}
          draft: false
          prerelease: false
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release creation job with auto-generated notes"
```

**Deliverable:** On tag push, all build artifacts are downloaded and uploaded as GitHub Release assets with auto-generated release notes.

---

### Task 5: Add RustDoc Publishing Job

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `quality` job (must pass first)
- Produces: Deployed RustDoc to `gh-pages` branch via `peaceiris/actions-gh-pages@v4`

Add the `rustdoc` job that builds workspace documentation and deploys to GitHub Pages. Only runs on `v*` tag pushes.

- [ ] **Step 1: Add rustdoc job to release.yml**

Insert after the `build` job (before `release`):

```yaml
  rustdoc:
    name: build-and-deploy-docs
    runs-on: ubuntu-latest
    needs: quality
    if: startsWith(github.ref, 'refs/tags/')
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Build documentation
        run: cargo doc --workspace --no-deps

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc
          force_orphan: false
          keep_files: true
```

Note: `cargo doc --workspace --no-deps` outputs to `target/doc/` by default. The `keep_files: true` ensures previous version docs aren't overwritten.

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add RustDoc publishing to GitHub Pages on tag releases"
```

**Deliverable:** On tag push, RustDoc is built for the entire workspace and deployed to the `gh-pages` branch.

---

### Task 6: Test and Verify Workflows

**Files:**
- No code changes — validation only

**Interfaces:** N/A

Verify both workflows are syntactically valid and the release workflow has correct job dependencies.

- [ ] **Step 1: Validate workflow YAML syntax**

Run locally to verify no YAML errors:

```bash
# Check for YAML syntax errors (requires python)
python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

If `pyyaml` isn't installed: `pip install pyyaml`

- [ ] **Step 2: Verify job dependency chain in release.yml**

Confirm the dependency graph:
- `quality` → runs first, no dependencies
- `version-check` → `needs: quality`, only on tags
- `build` (matrix) → `needs: quality`
- `rustdoc` → `needs: quality`, only on tags
- `release` → `needs: build`, only on tags

- [ ] **Step 3: Commit any fixes**

If validation finds issues, fix and commit. Otherwise, the workflows are ready.

**Deliverable:** Both workflows validated — correct YAML syntax, correct job dependencies, correct trigger conditions.
