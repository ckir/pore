mod common;
use common::*;

use pore_core::{IndexOptions, SearchOptions};
use std::collections::HashMap;

#[test]
fn add_and_search_documents() {
    let (_tmp, mut index) =
        create_test_generic_index("id", &["title", "body"], IndexOptions::default());

    let mut doc1 = HashMap::new();
    doc1.insert("id".to_string(), "1".to_string());
    doc1.insert("title".to_string(), "Hello World".to_string());
    doc1.insert("body".to_string(), "This is a test document".to_string());

    let mut doc2 = HashMap::new();
    doc2.insert("id".to_string(), "2".to_string());
    doc2.insert("title".to_string(), "Goodbye World".to_string());
    doc2.insert("body".to_string(), "Another document".to_string());

    index.add_documents(vec![doc1, doc2]).unwrap();

    use tantivy::query::QueryParser;
    let query_parser = QueryParser::for_index(index.index(), index.get_text_fields());
    let query = query_parser.parse_query("Hello").unwrap();
    let results = index.search(&query, &SearchOptions::default()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

#[test]
fn delete_documents_by_id() {
    let (_tmp, mut index) = create_test_generic_index("id", &["text"], IndexOptions::default());

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "hello world".to_string());
    index.add_documents(vec![doc]).unwrap();

    index.delete_documents(vec!["1".to_string()]).unwrap();

    use tantivy::query::QueryParser;
    let query_parser = QueryParser::for_index(index.index(), index.get_text_fields());
    let query = query_parser.parse_query("hello").unwrap();
    let results = index.search(&query, &SearchOptions::default()).unwrap();
    assert!(results.is_empty());
}

#[test]
fn update_documents_replaces_fields() {
    let (_tmp, mut index) = create_test_generic_index("id", &["text"], IndexOptions::default());

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "original content".to_string());
    index.add_documents(vec![doc]).unwrap();

    let mut updated = HashMap::new();
    updated.insert("id".to_string(), "1".to_string());
    updated.insert("text".to_string(), "updated content".to_string());
    index.update_documents(vec![updated]).unwrap();

    use tantivy::query::QueryParser;
    let query_parser = QueryParser::for_index(index.index(), index.get_text_fields());
    let query = query_parser.parse_query("original").unwrap();
    let results = index.search(&query, &SearchOptions::default()).unwrap();
    assert!(results.is_empty());

    let query = query_parser.parse_query("updated").unwrap();
    let results = index.search(&query, &SearchOptions::default()).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn delete_nonexistent_id_is_noop() {
    let (_tmp, mut index) = create_test_generic_index("id", &["text"], IndexOptions::default());
    index
        .delete_documents(vec!["nonexistent".to_string()])
        .unwrap();
}

#[test]
fn empty_index_returns_no_results() {
    let (_tmp, index) = create_test_generic_index("id", &["text"], IndexOptions::default());
    use tantivy::query::QueryParser;
    let query_parser = QueryParser::for_index(index.index(), index.get_text_fields());
    let query = query_parser.parse_query("anything").unwrap();
    let results = index.search(&query, &SearchOptions::default()).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_with_limit() {
    let (_tmp, mut index) = create_test_generic_index("id", &["text"], IndexOptions::default());

    for i in 0..5 {
        let mut doc = HashMap::new();
        doc.insert("id".to_string(), i.to_string());
        doc.insert("text".to_string(), format!("test document number {}", i));
        index.add_documents(vec![doc]).unwrap();
    }

    let query_parser =
        tantivy::query::QueryParser::for_index(index.index(), index.get_text_fields());
    let query = query_parser.parse_query("test").unwrap();
    let opts = SearchOptions {
        limit: 2,
        ..Default::default()
    };
    let results = index.search(&query, &opts).unwrap();
    assert!(results.len() <= 2);
}

#[test]
fn search_with_threshold() {
    let (_tmp, mut index) = create_test_generic_index("id", &["text"], IndexOptions::default());

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "hello".to_string());
    index.add_documents(vec![doc]).unwrap();

    let query_parser =
        tantivy::query::QueryParser::for_index(index.index(), index.get_text_fields());
    let query = query_parser.parse_query("hello").unwrap();
    let opts = SearchOptions {
        threshold: 0.0,
        ..Default::default()
    };
    let results = index.search(&query, &opts).unwrap();
    assert_eq!(results.len(), 1);
}
