## Task 1: Rewrite CI Workflow with Full Quality Gate

**Status:** DONE

**Commit:** `245edb3` — `ci: rewrite with parallel quality gate (fmt, clippy, test)`

**Test summary:** YAML validated (3 parallel jobs: fmt, clippy, test — all ubuntu-latest, stable via dtolnay/rust-toolchain); workflow committed.

**Changes:**
- Replaced old matrix-based CI (stable/beta/nightly across ubuntu-20.04, macos-latest, windows-2019) with three parallel jobs
- `actions-rs/toolchain@v1` → `dtolnay/rust-toolchain@stable`
- `actions/checkout@v2` → `actions/checkout@v4`
- `fmt`: checkout + rustfmt component + `cargo fmt --all -- --check`
- `clippy`: checkout + clippy component + `cargo clippy --workspace --all-targets -- -D warnings`
- `test`: checkout + stable toolchain + `cargo test --workspace --verbose` (with `RUST_BACKTRACE: 1`)
- All jobs trigger on push to master and pull_request
- Net diff: +25 -47 lines

**Concerns:** None.
