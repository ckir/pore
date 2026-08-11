# Task 5 Report: Add rustdoc job to release workflow

## Status: DONE

## Commits
- `2b6b0c9` — ci: add rustdoc job to release workflow

## Test summary
Workflow YAML structure verified; rustdoc job correctly inserted between build and release with all required fields.

## Changes
- Added `rustdoc` job to `.github/workflows/release.yml` after `build`, before `release`
- Job runs `cargo doc --workspace --no-deps` → publishes `target/doc/` to `gh-pages` branch via `peaceiris/actions-gh-pages@v4`
- `keep_files: true` preserves previous version docs
- `permissions: contents: write` enables the GitHub Pages push
- `if: startsWith(github.ref, 'refs/tags/')` restricts to tag pushes only
- Updated `release` job `needs` to include `rustdoc` so release waits for docs deployment

## Concerns
None.
