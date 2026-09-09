# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **Core Indexing**: Support for native Regular Expressions and Field Grouping in search queries (leveraging Tantivy 0.26 capabilities).
- **Sorting**: `--sort date` and `--sort path` now work. The flags were previously declared but wired to nothing, so they silently did nothing. Backed by FAST fields and a `modified` timestamp in the index schema.
- **Snippets**: `--snippets` returns Tantivy-generated snippets (a condensed extract with the match highlighted) instead of matching lines. Opt-in; `lines` remains the default.
- **Documentation**: Comprehensive `ROADMAP.md` tracking all upcoming features.

### Changed
- **Performance**: Integrated zero-copy indexing for `PoreFileEntry`, dramatically reducing string allocations during `index.update()`.
- **Index size**: text fields are now `STORED`, which snippet generation requires. This grows the on-disk index; run `pore search --rebuild` once after upgrading.

### Notes
- The JSON output contract is unchanged by default: results still carry `lines` (with 1-based `number` and `text`). `snippets` appears only under `--snippets`, and the two are mutually exclusive — existing `--jq` filters over `.lines` keep working.
