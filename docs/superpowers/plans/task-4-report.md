# Task 4 Report: CI Release Job

## Status: DONE

## Commit

- **31298cf** — `ci: add release job with artifact download and git log release notes`

## Test Summary

YAML syntax validated; workflow has 4 jobs (quality, version-check, build, release) with correct dependency chain.

## Changes

**Modified:** `.github/workflows/release.yml` (38 lines appended)

The `release` job:
- Runs on `ubuntu-latest` with `if: startsWith(github.ref, 'refs/tags/')`
- Depends on `quality` and `build` jobs via `needs: [quality, build]`
- Requests `contents: write` permission for release creation
- Checks out with `fetch-depth: 0` for full git history (needed by `git describe`)
- Downloads all build artifacts via `actions/download-artifact@v4` to `dist/`
- Generates release body from `git log --oneline` between the previous tag and HEAD (falls back to "Initial release" when no prior tag exists)
- Creates the GitHub Release via `softprops/action-gh-release@v3` with `draft: false`, `prerelease: false`, and `files: dist/**/*`

## Concerns

None.
