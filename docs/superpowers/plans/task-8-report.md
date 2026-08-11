# Task 8 Report: pore-bin CLI integration tests

## Status: DONE

## Commit

**SHA:** `a935e09`
**Subject:** `test: add CLI integration tests with assert_cmd`

## Test Summary

8 integration tests, all passing on Windows:

| Test | Description |
|------|-------------|
| `help_exits_zero` | `--help` exits 0 |
| `no_args_exits_zero` | No args exits 0 (no query) |
| `files_command_lists_files` | `--files` lists files in target directory |
| `indexes_command_prints_index_info` | `--indexes` prints index metadata |
| `delete_command_exits_zero` | `--delete` exits 0 |
| `search_command_finds_matches` | Search finds matching files |
| `json_output_flag` | `--json` outputs JSON with `"file"` key |
| `filename_only_flag` | `-l` prints filenames only |

## Changes Made

### 1. `pore-bin/Cargo.toml`
Added `assert_cmd = "2"` and `predicates = "3"` to `[dev-dependencies]`.

### 2. `pore-bin/tests/cli_integration.rs` (new)
8 integration tests using `assert_cmd::Command` and `predicates`. Tests use `--in-memory` to avoid disk index path issues and set `HOME` env var for Windows compatibility (config file lookup requires it).

### 3. `pore-bin/src/args.rs` (bug fix)
Added `.action(clap::ArgAction::SetTrue)` to all 16 boolean flag args (`--in-memory`, `--rebuild`, `--files`, `--indexes`, `--delete`, `--json`, `-l`, etc.). Clap 4 requires explicit action for boolean flags; without it, the CLI would error with "a value is required for '--flag' but none was supplied".

### 4. `pore-bin/src/main.rs` (bug fix)
Fixed `find_index_dir` Windows path handling. The original `strip_prefix("/")` fails on Windows because paths start with `C:\`, not `/`. Replaced with a `strip_root()` helper that uses `path.components().skip(n)` where n=1 on Unix and n=2 on Windows (Prefix + RootDir).

## Concerns

- **Pre-existing test failures:** `config::tests::can_load_and_merge_defaults` and `config::tests::example_file_is_complete` fail due to a TOML parsing issue (`"unexpected content, expected nothing"`). This is unrelated to this task and was present before these changes.
- **Tests use `--in-memory`:** To avoid the Windows disk index path bug (which is now fixed), tests use `--in-memory` flag. Tests that specifically need on-disk indexes could now be written but were kept simple with in-memory for portability.
- **`HOME` env var required:** Tests set `HOME` to a temp directory because pore's config loading requires `HOME` or `XDG_CONFIG_HOME`. On Windows, `HOME` is not set by default.
