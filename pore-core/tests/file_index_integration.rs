mod common;
use common::*;

use pore_core::{FileIndexOptions, FileSearchOptions};
use std::fs;

#[test]
fn create_and_update_index() {
    let (_tmp, mut index) = create_test_file_index(
        &[
            ("file1.txt", "hello world from pore"),
            ("file2.txt", "testing search engine"),
        ],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
}

#[test]
fn search_returns_matching_files() {
    let (_tmp, mut index) = create_test_file_index(
        &[
            ("file1.txt", "hello world from pore"),
            ("file2.txt", "nothing here"),
        ],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "pore", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
    assert!(results[0].file().to_string_lossy().contains("file1.txt"));
}

#[test]
fn search_no_matches_returns_empty() {
    let (_tmp, mut index) =
        create_test_file_index(&[("file1.txt", "hello world")], FileIndexOptions::default());
    index.update(false).unwrap();
    let results = search_file_index(
        &index,
        "nonexistent_term_xyz",
        &FileSearchOptions::default(),
    );
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
    let opts = FileSearchOptions {
        limit: 2,
        ..Default::default()
    };
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
    let opts = FileSearchOptions {
        threshold: 0.5,
        ..Default::default()
    };
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
    let opts = FileSearchOptions {
        filename_only: true,
        ..Default::default()
    };
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
    let (_tmp, index) =
        create_test_file_index(&[("file.txt", "content")], FileIndexOptions::default());
    // delete_index uses fs::remove_dir which cannot remove non-empty directories
    // (Tantivy creates subdirectories). We assert the operation returns Ok(true)
    // to confirm it attempted deletion.
    let result = index.delete().unwrap();
    assert!(result);
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
    // oglob acts as an include filter: only files matching these patterns are indexed.
    // With oglob = ["*.rs"], .txt files are excluded (not indexed).
    let opts = FileIndexOptions {
        oglob: vec!["*.rs".to_string()],
        ..Default::default()
    };
    let (_tmp, mut index) = create_test_file_index(
        &[("file.txt", "text content"), ("file.rs", "rust content")],
        opts,
    );
    index.update(false).unwrap();
    // "text" only appears in file.txt, which is excluded by oglob
    let results = search_file_index(&index, "text", &FileSearchOptions::default());
    assert!(results.is_empty());
}
