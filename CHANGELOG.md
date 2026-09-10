# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **Core Indexing**: Support for native Regular Expressions and Field Grouping in search queries (leveraging Tantivy 0.26 capabilities).
- **Sorting**: `--sort date` and `--sort path` now work. The flags were previously declared but wired to nothing, so they silently did nothing. Backed by FAST fields and a `modified` timestamp in the index schema.
- **Aggregation**: `--aggregate <field>` now works. Like `--sort`, the flag was previously declared but wired to nothing. It groups the documents matching the query into buckets by a FAST field (`ext`, `filepath`, `modified`) and prints Tantivy's terms-aggregation counts as JSON; `--jq` composes with it. Aggregating on a missing or non-fast field fails with an error naming the field.
- **Regex queries**: `allow_regexes()` is now enabled on the CLI's query parser, so `contents:/w.lf/` and `filepath:/.*\.rs/` work. Previously the capability existed only inside a test and every regex query was rejected with "Regex queries are not allowed".
- **Snippets**: `--snippets` returns Tantivy-generated snippets (a condensed extract with the match highlighted) instead of matching lines. Opt-in; `lines` remains the default.
- **Documentation**: A Lua module section in the README — how to build a loadable module (it needs `--no-default-features --features lua55,module`; the default `vendored` build is not loadable), the `get_file_index` / `get_index` API, and the result shapes. The module was previously undocumented.
- **Documentation**: Comprehensive `ROADMAP.md` tracking all upcoming features.

### Fixed
- **Error output**: every failing command printed a stray `<disabled>` line after the message. `main` printed `err.backtrace()` unconditionally, and that is how `std::backtrace::Backtrace` renders when capture is off — the default. The backtrace is now printed only when it was actually captured, and via `Display` rather than `Debug`, so `RUST_BACKTRACE=1` gives a readable numbered trace instead of one long line.
- **Windows**: pore hard-required `HOME`, which Windows does not set, so every command that touched the config file or the index cache failed with a bare `environment variable not found` — only `--help` and `--version` worked. Both lookups now fall back to `%USERPROFILE%` after `HOME`, and when nothing resolves the error names every variable it consulted instead of propagating `VarError::NotPresent`.

### Changed
- **Performance**: Integrated zero-copy indexing for `PoreFileEntry`, dramatically reducing string allocations during `index.update()`.
- **Index size**: text fields are now `STORED`, which snippet generation requires. This grows the on-disk index; run `pore search --rebuild` once after upgrading.

### Notes
- The JSON output contract is unchanged by default: results still carry `lines` (with 1-based `number` and `text`). `snippets` appears only under `--snippets`, and the two are mutually exclusive — existing `--jq` filters over `.lines` keep working.
