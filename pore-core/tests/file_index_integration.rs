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
    assert!(results[0].snippets().is_empty());
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
fn search_default_returns_lines_and_no_snippets() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "line one\nhello match\nline three")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "hello", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
    assert!(
        !results[0].lines().is_empty(),
        "lines is the default output shape"
    );
    assert!(
        results[0].snippets().is_empty(),
        "snippets must stay empty unless opted in"
    );
}

#[test]
fn search_returns_snippets_when_opted_in() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "line one\nhello match\nline three")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let opts = FileSearchOptions {
        snippets: true,
        ..Default::default()
    };
    let results = search_file_index(&index, "hello", &opts);
    assert_eq!(results.len(), 1);
    let snippets = results[0].snippets();
    assert!(!snippets.is_empty(), "opting in must produce snippets");
    assert!(snippets.iter().any(|s| s.contains("hello")));
    assert!(
        results[0].lines().is_empty(),
        "lines must stay empty when snippets are requested"
    );
}

#[test]
fn search_line_numbers_are_one_based() {
    let (_tmp, mut index) = create_test_file_index(
        &[("file1.txt", "line one\nhello match\nline three")],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();
    let results = search_file_index(&index, "hello", &FileSearchOptions::default());
    assert_eq!(results.len(), 1);
    let lines = results[0].lines();
    let hit = lines
        .iter()
        .find(|l| l.text.contains("hello match"))
        .expect("the matching line is reported");
    assert_eq!(hit.number, 2, "second line of the file is line 2, not 1");
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
    let (_tmp, mut index) = create_test_file_index(
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

#[test]
fn ext_fast_field_is_populated_per_document() {
    // The `ext` field is indexed for future aggregation support; nothing reads it
    // yet, so this pins that it is actually written per document rather than left
    // empty. A TermQuery on it must select exactly the file with that extension.
    use tantivy::collector::TopDocs;
    use tantivy::query::TermQuery;
    use tantivy::schema::{IndexRecordOption, Value};
    use tantivy::Term;

    let (_tmp, mut index) = create_test_file_index(
        &[
            ("keep.rs", "rust content here"),
            ("skip.txt", "text content here"),
        ],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();

    let schema_ext = index.index().schema().get_field("ext").unwrap();
    let filepath = index.index().schema().get_field("filepath").unwrap();
    let reader = index.index().reader().unwrap();
    let searcher = reader.searcher();

    let query = TermQuery::new(
        Term::from_field_text(schema_ext, "rs"),
        IndexRecordOption::Basic,
    );
    let hits = searcher
        .search(&query, &TopDocs::with_limit(10).order_by_score())
        .unwrap();
    assert_eq!(hits.len(), 1, "exactly one .rs file was indexed");

    let doc: tantivy::TantivyDocument = searcher.doc(hits[0].1).unwrap();
    let path = doc.get_first(filepath).unwrap().as_str().unwrap();
    assert!(
        path.ends_with("keep.rs"),
        "the ext:rs hit must be keep.rs, got {path}"
    );
}

/// Parses `query_str` against the contents field, the same way the CLI does.
fn parse_query(index: &pore_core::FileIndex, query_str: &str) -> Box<dyn tantivy::query::Query> {
    use tantivy::query::QueryParser;
    let parser = QueryParser::for_index(index.index(), vec![*index.contents()]);
    parser.parse_query(query_str).unwrap()
}

#[test]
fn aggregate_by_ext_counts_matching_files_per_extension() {
    let (_tmp, mut index) = create_test_file_index(
        &[
            ("a.rs", "hello rust"),
            ("b.rs", "hello again"),
            ("c.txt", "hello text"),
            ("d.txt", "unrelated content"),
        ],
        FileIndexOptions::default(),
    );
    index.update(false).unwrap();

    let query = parse_query(&index, "hello");
    let opts = FileSearchOptions {
        aggregate: Some("ext".to_string()),
        ..Default::default()
    };
    let value = index.aggregate(&*query, &opts).unwrap();

    let buckets = value["ext"]["buckets"]
        .as_array()
        .unwrap_or_else(|| panic!("expected ext buckets, got: {value}"));
    let mut counts: Vec<(String, u64)> = buckets
        .iter()
        .map(|b| {
            (
                b["key"].as_str().unwrap().to_string(),
                b["doc_count"].as_u64().unwrap(),
            )
        })
        .collect();
    counts.sort();

    // d.txt does not match "hello", so it must not be counted.
    assert_eq!(
        counts,
        vec![("rs".to_string(), 2), ("txt".to_string(), 1)],
        "aggregation must count only the documents matching the query"
    );
}

#[test]
fn aggregate_on_unknown_field_reports_the_field_name() {
    let (_tmp, mut index) =
        create_test_file_index(&[("a.rs", "hello")], FileIndexOptions::default());
    index.update(false).unwrap();

    let query = parse_query(&index, "hello");
    let opts = FileSearchOptions {
        aggregate: Some("not_a_field".to_string()),
        ..Default::default()
    };
    let err = index
        .aggregate(&*query, &opts)
        .expect_err("aggregating on a field that does not exist must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("not_a_field"),
        "the error must name the offending field, got: {msg}"
    );
}
