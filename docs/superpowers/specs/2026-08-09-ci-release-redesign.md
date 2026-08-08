---
name: ci-release-redesign
description: Design for replacing CI workflows with tag-based release builds producing pre-compiled binaries for Windows/Linux/macOS
---

# CI Release Redesign

## Overview

Replace the current multi-channel CI (stable/beta/nightly across OSes) with two workflows:
1. **CI** — runs on push to master and PRs: format check + test
2. **Release** — triggered by push tags or manual dispatch: builds release binaries, creates GitHub Releases on tag pushes

## Trigger Model

| Trigger | CI Jobs | Release Jobs |
|---------|---------|-------------|
| Push to master | ✅ run | ❌ skip (artifacts kept 1 day) |
| PR | ✅ run | ❌ skip |
| Push tag `v*` | ✅ run | ✅ run (creates GitHub Release) |
| Manual dispatch | ❌ skip | ✅ run (artifacts kept 1 day) |

## Version Extraction

- **Tag push:** version = tag name minus leading `v` (e.g., `v0.2.0` → `0.2.0`). Must match `Cargo.toml` version or fail early.
- **Manual dispatch / non-tag push:** version = `Cargo.toml` version + `-<short-sha>` suffix for artifact naming.

## Build Matrix

Approach B: native builds on latest runners, one target per matrix entry (macOS builds both Darwin targets).

| Runner | Target Triple | Archive Format |
|--------|--------------|----------------|
| `ubuntu-latest` | `x86_64-unknown-linux-gnu` | `.tar.gz` + `.sha256` |
| `macos-latest` | `x86_64-apple-darwin` | `.tar.gz` + `.sha256` |
| `macos-latest` | `aarch64-apple-darwin` | `.tar.gz` + `.sha256` |
| `windows-latest` | `x86_64-pc-windows-msvc` | `.zip` + `.sha256` |

### Archive Contents

Each archive contains:
- `pore` binary (platform-specific extension)
- `README.md`
- `LICENSE`

Checksum file: `{archive-name}.sha256`

## Workflow Structure

### CI Workflow (`.github/workflows/ci.yml`)

**Jobs:**
- `rustfmt` — format check on `ubuntu-latest`
- `test` — `cargo test` on `ubuntu-latest` (stable Rust only)

Removed: beta/nightly channels, redundant OS test matrix. Format + test on stable Linux covers the logic; platform-specific behavior is exercised by the release build jobs.

### Release Workflow (`.github/workflows/release.yml`)

**Jobs:**
- `build` (matrix) — 4 entries, one per target triple. Each:
  1. Install Rust stable
  2. `rustup target add <triple>`
  3. `cargo build --release --target <triple>`
  4. Package binary + README + LICENSE into archive (`.tar.gz` for Unix, `.zip` for Windows)
  5. Generate SHA256 checksum
  6. Upload artifact (retention: 1 day)

- `release` — depends on all `build` jobs. Only runs on `v*` tag push.
  1. Download all artifacts
  2. Create GitHub Release (`softprops/action-gh-release@v2`)
  3. Upload archives + checksums as release assets
  4. Release body from `git log` since previous tag (or "Initial release")

## Actions (Latest Versions)

- `actions/checkout@v4`
- `actions-rs/toolchain@v1` (or `dtolnay/rust-toolchain@stable` if preferred)
- `actions/upload-artifact@v4`
- `actions/download-artifact@v4`
- `softprops/action-gh-release@v2`

## Artifact Retention

- Workflow artifacts: 1 day (`retention-days: 1`)
- GitHub Release assets: permanent (only created on tag push)
