# Task 6 Report: pore-core location unit tests

## Status: DONE

## Commit
- **b0a7b93** — `test: add location position-to-line unit tests`

## Test Summary
All 4 location unit tests pass (0.07s):
- `positions_to_lines_empty_positions_produces_no_lines` — verifies empty positions returns Ok with no lines
- `positions_to_lines_empty_file` — verifies empty file with empty positions returns Ok
- `positions_to_lines_single_line` — verifies a single-position file reads 1 line correctly
- `positions_to_lines_multi_line` — verifies a 3-line file with one position reads all 3 lines

## Changes Made
- Appended `#[cfg(test)] mod tests` to `pore-core/src/location.rs` (92 lines added)
- Added `use std::fs;` import (not re-exported by `super::*` — only `std::fs::File` was)
- Fixed the brief's helper function: corrected invalid `while let Ok(bytes) = reader.read_line(&mut line_str)?` syntax to a `loop { let bytes = reader.read_line(&mut line_str)?; ... }` pattern

## Concerns
None. The tests use the simplified `positions_to_lines_no_index` helper as intended by the brief — testing line-counting logic without requiring a full `FileIndex`. The full `positions_to_lines` function is covered through FileIndex integration tests (Task 4).
