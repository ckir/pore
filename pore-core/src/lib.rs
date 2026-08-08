//! Core library for pore — a full-text search engine built on Tantivy.
//!
//! This crate provides the foundational types and abstractions for building
//! and querying text indexes. It supports two index types:
//!
//! - [`file::FileIndex`] — indexes files in a directory tree, with automatic
//!   file-walking (respecting .gitignore), content indexing, and line-level
//!   search result highlighting.
//! - [`generic::GenericIndex`] — a generic key-value document index where each
//!   document is a map of named text fields.
//!
//! # Re-exports
//!
//! The following modules are re-exported for convenience:
//! - [`field_map`] — the [`field_map::FieldMap`] trait for extracting field values
//! - [`file`] — file-based indexing types
//! - [`generic`] — generic document indexing types
//!
//! The [`language`] module is also public but not re-exported.

#[macro_use]
extern crate anyhow;

mod common;
mod field_map;
mod file;
mod generic;
pub mod language;
mod location;

pub use field_map::*;
pub use file::*;
pub use generic::*;
