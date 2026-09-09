//! Core library for pore — a full-text search engine built on Tantivy.
//!
//! This crate provides the foundational types and abstractions for building
//! and querying text indexes. It supports two index types:
//!
//! - [`FileIndex`] — indexes files in a directory tree, with automatic
//!   file-walking (respecting .gitignore), content indexing, and line-level
//!   search result highlighting.
//! - [`GenericIndex`] — a generic key-value document index where each
//!   document is a map of named text fields.
//!
//! # Re-exports
//!
//! The following are re-exported for convenience:
//! - [`FieldMap`] — trait for extracting field values from data sources
//! - [`FileIndex`], [`FileIndexOptions`], [`FileSearchOptions`], [`FileSearchResult`]
//! - [`GenericIndex`], [`IndexOptions`], [`SearchOptions`], [`SearchResult`]
//! - [`Line`] — a single matching line in search results
//!
//! The [`language`] module is also public but not re-exported.

#[macro_use]
extern crate anyhow;

mod common;
mod field_map;
mod file;
mod generic;
pub mod jq;
pub mod language;
mod location;

pub use field_map::*;
pub use file::*;
pub use generic::*;
