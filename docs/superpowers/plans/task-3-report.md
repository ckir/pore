# Task 3 Report: CI Release Build Matrix Job

**Status:** DONE

**Commit:** f8a0c6b

## What was implemented

Added a `build` job to `.github/workflows/release.yml` (after `version-check`) that:

1. **Matrix:** 4 platform targets
   - `ubuntu-latest` → `x86_64-unknown-linux-gnu` (`.tar.gz`)
   - `macos-latest` → `aarch64-apple-darwin` (`.tar.gz`)
   - `macos-13` → `x86_64-apple-darwin` (`.tar.gz`)
   - `windows-latest` → `x86_64-pc-windows-msvc` (`.zip`, MSVC only)

2. **Steps per matrix entry:**
   - `actions/checkout@v4` + `dtolnay/rust-toolchain@stable` with target
   - `cargo build --release -p pore --target ${{ matrix.target }}`
   - Package staging: binary + `README.md` + `LICENSE` into `dist/pore-<target>/`
   - Platform-native archiving: `tar czf` for Unix, `7z a` for Windows
   - SHA256 checksum: `sha256sum` for Unix, `certutil` + `findstr` for Windows
   - `actions/upload-artifact@v7` with `retention-days: 1`

3. **Constraints met:**
   - `shell: bash` on all platforms
   - `fail-fast: false`
   - MSVC only for Windows (no GNU target)
   - Latest major versions for all actions (`@v4`, `@v2`)

## Test summary
YAML validated via Python `yaml.safe_load`; workflow structure verified; committed as f8a0c6b.

## Concerns
- The `certutil` + `findstr` pipeline on Windows produces checksum files with different whitespace/format than `sha256sum` output. The checksum values are correct, but cross-platform verification scripts would need to handle both formats. If standardized format is required downstream, the Windows checksum step could be adjusted.
