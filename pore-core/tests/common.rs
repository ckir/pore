use pore_core::{FileIndex, FileIndexOptions, GenericIndex, IndexOptions};
use std::fs;
use std::path::Path;
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
