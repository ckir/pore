### Task 4: pore-core FileIndex unit and integration tests

**Files:**
- Modify: `pore-core/src/file.rs` (append `#[cfg(test)]` module)
- Create: `pore-core/tests/common.rs`
- Create: `pore-core/tests/file_index_integration.rs`

**Interfaces:**
- Consumes: `FileIndex`, `FileIndexOptions`, `FileSearchOptions`, `FileMetadata`
- Produces: shared test helpers

- [ ] **Step 1: Add tempfile dev-dependency** (already done in Task 1 — skip if present)

- [ ] **Step 2: Write FileIndex unit tests**

Append to `pore-core/src/file.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn file_index_options_default() {
        let opts = FileIndexOptions::default();
        assert!(!opts.follow);
        assert!(!opts.hidden);
        assert!(opts.ignore_files);
        assert_eq!(opts.language, LanguageRef::English);
        assert_eq!(opts.threads, 0);
    }

    #[test]
    fn file_search_options_default() {
        let opts = FileSearchOptions::default();
        assert_eq!(opts.limit, 1000);
        assert_eq!(opts.threshold, 0.0);
        assert!(!opts.filename_only);
        assert!(opts.root_dir.is_none());
    }

    #[test]
    fn file_metadata_new() {
        let tmp = TempDir::new().unwrap();
        let opts = FileIndexOptions::default();
        let meta = FileMetadata::new(opts.clone(), tmp.path()).unwrap();
        assert_eq!(meta.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(meta.for_dir(), fs::canonicalize(tmp.path()).unwrap());
    }

    #[test]
    fn file_metadata_serialization_round_trip() {
        let tmp = TempDir::new().unwrap();
        let opts = FileIndexOptions::default();
        let meta = FileMetadata::new(opts, tmp.path()).unwrap();
        let json = serde_json::to_string(&meta).unwrap();
        let restored: FileMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta.version, restored.version);
        assert_eq!(meta.config, restored.config);
    }

    #[test]
    fn file_index_display_format() {
        let tmp = TempDir::new().unwrap();
        let opts = FileIndexOptions::default();
        let index = FileIndex::get_or_create(tmp.path(), Some(tmp.path()), &opts).unwrap();
        let display = format!("{}", index);
        assert!(display.contains("Index("));
        assert!(display.contains("version:"));
        assert!(display.contains("location:"));
    }
}
```

- [ ] **Step 3: Create shared test helpers**

Create `pore-core/tests/common.rs`:

```rust
use pore_core::{FileIndex, FileIndexOptions, GenericIndex, IndexOptions, Line};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Create test files in a directory with given (relative_path, content) pairs.
pub fn create_test_files(dir: &Path, files: &[(&str, &str)]) {
    for (rel_path, content) in files {
        let full_path = dir.join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(full_path, content).unwrap();
    }
}

/// Create a FileIndex for a test directory with the given options.
pub fn create_test_file_index(
    files: &[(&str, &str)],
    opts: FileIndexOptions,
) -> (TempDir, FileIndex) {
    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), files);
    let index = FileIndex::get_or_create(tmp.path(), Some(tmp.path()), &opts).unwrap();
    (tmp, index)
}

/// Create a GenericIndex for testing.
pub fn create_test_generic_index(
    id_field: &str,
    text_fields: &[&str],
    opts: IndexOptions,
) -> (TempDir, GenericIndex) {
    let tmp = TempDir::new().unwrap();
    let index =
        GenericIndex::get_or_create(id_field, text_fields.to_vec(), &opts, Some(tmp.path()))
            .unwrap();
    (tmp, index)
}

/// Parse a query string and run search on a FileIndex, returning results.
pub fn search_file_index(
    index: &FileIndex,
    query_str: &str,
    opts: &pore_core::FileSearchOptions,
) -> Vec<pore_core::FileSearchResult> {
    use tantivy::query::QueryParser;
    let query_parser = QueryParser::for_index(index.index(), vec![*index.contents()]);
    let query = query_parser.parse_query(query_str).unwrap();
    index.search(&query, opts).unwrap()
}
```

- [ ] **Step 4: Create FileIndex integration tests**

Create `pore-core/tests/file_index_integration.rs`:

```rust
mod common;
use common::*;

use pore_core::{FileIndexOptions, FileSearchOptions, LanguageRef};

#[test]
fn create_and_update_index() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "hello world from pore"), ("file2.txt", "testing search engine")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
}

#[test]
fn search_returns_matching_files() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "hello world from pore"), ("file2.txt", "nothing here")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "pore", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
    assert!(results[0].file().to_string_lossy().contains("file1.txt"));
}

#[test]
fn search_no_matches_returns_empty() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "hello world")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "nonexistent_term_xyz", &FileSearchOptions::default());
    assert!(results.is_empty());
}

#[test]
fn search_with_limit() {
    let (_tmp, mut index) = create_test_file_index(
        &[
            ("a.txt", "hello hello hello"),
            ("b.txt", "hello hello"),
            ("c.txt", "hello"),
        ],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let opts = FileSearchOptions { limit: 2, ..Default::default() };
    let results = search_file_index(&index, "hello", &opts);
    assert!(results.len() <= 2);
}

#[test]
fn search_with_threshold_filters() {
    let (_tmp, mut index) = create_test_file_index(
        &[("match.txt", "hello world"), ("weak.txt", "xyz")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let opts = FileSearchOptions { threshold: 0.5, ..Default::default() };
    let results = search_file_index(&index, "hello", &opts);
    for r in &results {
        assert!(r.score() >= 0.5);
    }
}

#[test]
fn search_filename_only_omits_lines() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "hello world matching line")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let opts = FileSearchOptions { filename_only: true, ..Default::default() };
    let results = search_file_index(&index, "hello", &opts);
    assert_eq!(results.len(), 1);
    assert!(results[0].lines().is_empty());
}

#[test]
fn search_returns_matching_lines() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "line one\nhello match\nline three")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let opts = FileSearchOptions::default();
    let results = search_file_index(&index, "hello", &opts);
    assert_eq!(results.len(), 1);
    let lines = results[0].lines();
    assert!(!lines.is_empty());
    assert!(lines.iter().any(|l| l.text.contains("hello match")));
}

#[test]
fn update_reindex_modified_files() {
    let (tmp, mut index) = create_test_file_index(
        &[("file.txt", "original content")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    fs::write(tmp.path().join("file.txt"), "new content added").unwrap();
    index.update(false).unwrap();
    let results = search_file_index(&index, "new", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
}

#[test]
fn update_rebuild_forces_full_reindex() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file.txt", "searchable content")],
        FileIndexOptions::default(),
    );
    index.update(true).unwrap();
    let results = search_file_index(&index, "searchable", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
}

#[test]
fn delete_index_removes_files() {
    let (tmp, index) = create_test_file_index(
        &[("file.txt", "content")],
        FileIndexOptions::default(),
    );
    index.delete().unwrap();
    assert!(!tmp.path().exists());
}

#[test]
fn file_walker_respects_hidden_toggle() {
    let (tmp, mut index) = create_test_file_index(
        &[("visible.txt", "visible"), (".hidden.txt", "hidden")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "visible", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
    let hidden_results = search_file_index(&index, "hidden", &FileSearchOptions::default());
    assert!(hidden_results.is_empty());
}

#[test]
fn file_walker_respects_glob_include() {
    let opts = FileIndexOptions {
        glob: vec!["*.rs".to_string()],
        ..Default::default()
    };
    let (_tmp, mut index) = create_test_file_index(
        &[("file.txt", "text content"), ("file.rs", "rust content")],
        opts,
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "rust", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
    assert!(results[0].file().to_string_lossy().ends_with(".rs"));
}

#[test]
fn file_walker_respects_glob_exclude() {
    let opts = FileIndexOptions {
        glob: vec!["!*.txt".to_string()],
        ..Default::default()
    };
    let (_tmp, mut index) = create_test_file_index(
        &[("file.txt", "text content"), ("file.rs", "rust content")],
        opts,
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "text", &FileSearchOptions::default());
    assert!(results.is_empty());
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p pore-core --features vendored -- --nocapture`
Expected: All unit + integration tests PASS

- [ ] **Step 6: Commit**

```bash
git add pore-core/src/file.rs pore-core/tests/common.rs pore-core/tests/file_index_integration.rs
git commit -m "test: add FileIndex unit and integration tests"
```
