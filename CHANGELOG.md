# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **Core Indexing**: Support for native Regular Expressions and Field Grouping in search queries (leveraging Tantivy 0.26 capabilities).
- **Documentation**: Comprehensive `ROADMAP.md` tracking all upcoming features.

### Changed
- **Performance**: Integrated zero-copy indexing for `PoreFileEntry`, dramatically reducing string allocations during `index.update()`.
