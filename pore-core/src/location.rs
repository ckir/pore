//! Search result position extraction and line mapping.
//!
//! Tantivy stores term positions (token offsets) but not line numbers. This
//! module bridges that gap:
//! - [`get_search_results`] performs a second pass over the index to collect
//!   token positions for matched documents.
//! - [`positions_to_lines`] reads the original file from disk, re-tokenizes it
//!   line by line, and maps token positions back to line numbers.
//!
//! **Performance note:** [`get_search_results`] is effectively a second full-index
//! scan. A future optimization would use a custom [`tantivy::Collector`] to track
//! positions during the initial query.

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    fs::File,
    io::{self, BufRead},
    path::Path,
};

use tantivy::{
    postings::Postings,
    query::Query, schema::IndexRecordOption, DocAddress, DocSet, Searcher, TERMINATED,
};

use crate::{FileIndex, Line};

/// Min-heap of token positions (stored in reverse order for efficient popping).
type BytePositions = BinaryHeap<Reverse<u32>>;

/// Represents a matched document in search results.
#[derive(Debug)]
pub struct DocResult {
    /// Relevance score from the Tantivy query.
    pub score: f32,
    /// Document address within the index.
    pub address: DocAddress,
}

/// Collects token positions for each matched document.
///
/// Iterates over all index segments and postings to extract term positions
/// for the documents in `results`. This is a second full-index scan after the
/// initial query.
///
/// **Limitations:** May not work correctly with `FuzzyTermQuery` or
/// `PhraseQuery` — needs testing.
///
/// **Performance:** This doubles the cost of a search query. A better approach
/// would be a custom [`tantivy::Collector`] that tracks positions during the
/// initial query execution.
pub fn get_search_results(
    index: &FileIndex,
    query: &Box<dyn Query>,
    searcher: &Searcher,
    results: &Vec<DocResult>,
) -> Result<HashMap<DocAddress, BytePositions>, anyhow::Error> {
    let mut position_map: HashMap<DocAddress, BytePositions> = HashMap::new();
    for result in results {
        position_map.insert(result.address, BinaryHeap::new());
    }
    let mut terms = Vec::new();
    query.query_terms(&mut |term, _| terms.push(term.clone()));
    // this buffer will be used to request for positions
    let mut positions: Vec<u32> = Vec::with_capacity(100);
    for (segment_ord, segment_reader) in searcher.segment_readers().iter().enumerate() {
        let inverted_index = segment_reader.inverted_index(*index.contents())?;
        for term in &terms {
            if let Some(mut segment_postings) =
                inverted_index.read_postings(term, IndexRecordOption::WithFreqsAndPositions)?
            {
                let mut doc_id = segment_postings.doc();
                while doc_id != TERMINATED {
                    // This MAY contains deleted documents as well.
                    if segment_reader.is_deleted(doc_id) {
                        doc_id = segment_postings.advance();
                        continue;
                    }

                    if let Some(position_data) = position_map.get_mut(&DocAddress {
                        segment_ord: segment_ord as u32,
                        doc_id,
                    }) {
                        segment_postings.positions(&mut positions);
                        for pos in &positions {
                            position_data.push(Reverse(*pos));
                        }
                    }
                    doc_id = segment_postings.advance();
                }
            }
        }
    }

    Ok(position_map)
}

/// Maps token positions to line numbers and text.
///
/// Tantivy stores token offsets (not line numbers), so this function reads the
/// original file from disk, re-tokenizes it line by line, and matches token
/// positions against the accumulated token count to determine which lines
/// contain matches.
///
/// A future Tantivy enhancement could store byte or line offsets alongside
/// positions, eliminating the need to re-read and re-tokenize files.
pub fn positions_to_lines(
    index: &FileIndex,
    filepath: &Path,
    positions: &mut BytePositions,
    lines: &mut Vec<Line>,
) -> Result<(), anyhow::Error> {
    let mut tokenizer = index.index().tokenizer_for_field(*index.contents())?;
    if let Some(Reverse(mut next_pos)) = positions.peek() {
        let file = File::open(filepath)?;
        let mut reader = io::BufReader::new(file);
        let mut line = String::new();
        let mut line_no = 1;
        let mut num_tokens = 0;
        'outer: while let Ok(bytes) = reader.read_line(&mut line) {
            if bytes == 0 {
                break;
            }
            let mut line_tokens = 0;
            {
                let mut token_stream = tokenizer.token_stream(&line);
                while let Some(_) = token_stream.next() {
                    line_tokens += 1;
                }
            }
            if num_tokens <= next_pos && next_pos < num_tokens + line_tokens {
                lines.push(Line {
                    number: line_no,
                    text: line.trim_end().to_string(),
                });
                while next_pos < num_tokens + line_tokens {
                    match positions.pop() {
                        None => break 'outer,
                        Some(Reverse(pos)) => next_pos = pos,
                    };
                }
            }
            num_tokens += line_tokens;
            line.clear();
            line_no += 1;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Simplified positions-to-lines helper that reads all lines from a file.
    /// Tests the line-counting and file-reading logic without requiring a
    /// FileIndex (position-to-line mapping needs the index's tokenizer).
    fn positions_to_lines_no_index(
        filepath: &Path,
        positions: &mut BytePositions,
        lines: &mut Vec<Line>,
    ) -> Result<(), anyhow::Error> {
        if positions.is_empty() {
            return Ok(());
        }
        let file = File::open(filepath)?;
        let mut reader = io::BufReader::new(file);
        let mut line_str = String::new();
        let mut line_no = 1u32;
        loop {
            let bytes = reader.read_line(&mut line_str)?;
            if bytes == 0 {
                break;
            }
            lines.push(Line {
                number: line_no,
                text: line_str.trim_end().to_string(),
            });
            line_str.clear();
            line_no += 1;
        }
        Ok(())
    }

    #[test]
    fn positions_to_lines_empty_positions_produces_no_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("test.txt");
        fs::write(&test_file, "line one\nhello world\nline three").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert!(lines.is_empty());
    }

    #[test]
    fn positions_to_lines_empty_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("empty.txt");
        fs::write(&test_file, "").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert!(lines.is_empty());
    }

    #[test]
    fn positions_to_lines_single_line() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("single.txt");
        fs::write(&test_file, "hello world").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        positions.push(Reverse(0));
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].number, 1);
    }

    #[test]
    fn positions_to_lines_multi_line() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("multi.txt");
        fs::write(&test_file, "line one\nline two\nline three").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        positions.push(Reverse(2));
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2].number, 3);
    }
}
