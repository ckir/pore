//! Result output formatting.
//!
//! This module provides [`print_results`], which formats and prints search results
//! to stdout. Results can be emitted as JSON or as human-readable text with colored
//! filenames and line numbers.

use std::io::Write;

use pore_core::FileSearchResult;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::config::SearchConfig;

/// Prints the search results to stdout.
///
/// When `conf.json` is true, each result is serialized as a JSON object (one per line).
/// Otherwise, results are printed as human-readable text with colored filenames (magenta)
/// and line numbers (green).
///
/// Returns `Ok(true)` if at least one result was printed, `Ok(false)` otherwise.
pub fn print_results(
    results: Vec<FileSearchResult>,
    conf: &SearchConfig,
) -> Result<bool, anyhow::Error> {
    let mut stdout = StandardStream::stdout(conf.color.clone().into());
    // TODO make colors configurable
    let mut filename_color = ColorSpec::new();
    filename_color.set_fg(Some(Color::Magenta));
    let default_color = ColorSpec::new();
    let mut line_number_color = ColorSpec::new();
    line_number_color.set_fg(Some(Color::Green));

    for (i, result) in results.iter().enumerate() {
        if conf.json {
            println!("{}", serde_json::to_string(&result)?);
        } else {
            stdout.set_color(&filename_color)?;
            writeln!(&mut stdout, "{}", result.file().to_string_lossy())?;
            for line in result.lines() {
                stdout.set_color(&line_number_color)?;
                write!(&mut stdout, "{}", line.number)?;
                stdout.set_color(&default_color)?;
                writeln!(&mut stdout, ":{}", line.text)?;
            }
            if !conf.filename_only {
                if i < results.len() - 1 {
                    println!("");
                }
            }
        }
    }
    Ok(results.len() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pore_core::{FileSearchResult, Line};
    use std::path::PathBuf;

    #[test]
    fn print_results_json_format() {
        let results = vec![FileSearchResult::new(
            PathBuf::from("test.txt"),
            0.5,
            vec![Line {
                number: 1,
                text: "hello".to_string(),
            }],
        )];
        let conf = SearchConfig {
            json: true,
            color: ColorMode::Never,
            ..SearchConfig::default()
        };
        let result = print_results(results, &conf);
        assert!(result.unwrap());
    }

    #[test]
    fn print_results_empty_returns_false() {
        let conf = SearchConfig::default();
        let result = print_results(vec![], &conf);
        assert!(!result.unwrap());
    }

    #[test]
    fn print_results_non_empty_returns_true() {
        let results = vec![FileSearchResult::new(
            PathBuf::from("test.txt"),
            0.5,
            vec![],
        )];
        let conf = SearchConfig::default();
        let result = print_results(results, &conf);
        assert!(result.unwrap());
    }
}
