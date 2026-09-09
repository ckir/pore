//! Result output formatting.
//!
//! This module provides [`print_results`], which formats and prints search results
//! to stdout. Results can be emitted as JSON or as human-readable text with colored
//! filenames and line numbers.

use std::io::Write;

use pore_core::FileSearchResult;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::config::SearchConfig;

/// Splits a Tantivy snippet into `(highlighted, text)` segments.
///
/// [`tantivy::snippet::Snippet::to_html`] wraps matches in `<b>` tags and HTML-escapes
/// the surrounding fragment. Terminals want neither, so this decodes the entities and
/// reports which spans were highlighted, leaving the colouring itself to `termcolor`.
fn snippet_segments(snippet: &str) -> Vec<(bool, String)> {
    let decode = |s: &str| {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#x27;", "'")
            .replace("&#39;", "'")
            // `&amp;` last, so `&amp;lt;` decodes to the literal `&lt;`.
            .replace("&amp;", "&")
    };
    let mut segments = Vec::new();
    let mut rest = snippet;
    while let Some(open) = rest.find("<b>") {
        if open > 0 {
            segments.push((false, decode(&rest[..open])));
        }
        rest = &rest[open + 3..];
        match rest.find("</b>") {
            Some(close) => {
                segments.push((true, decode(&rest[..close])));
                rest = &rest[close + 4..];
            }
            // Unterminated tag: treat the remainder as highlighted rather than dropping it.
            None => {
                segments.push((true, decode(rest)));
                rest = "";
                break;
            }
        }
    }
    if !rest.is_empty() {
        segments.push((false, decode(rest)));
    }
    segments
}

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
    let mut highlight_color = ColorSpec::new();
    highlight_color.set_fg(Some(Color::Red));

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
            for snippet in result.snippets() {
                write!(&mut stdout, "  ")?;
                for (highlighted, text) in snippet_segments(snippet) {
                    if highlighted {
                        stdout.set_color(&highlight_color)?;
                    } else {
                        stdout.set_color(&default_color)?;
                    }
                    write!(&mut stdout, "{text}")?;
                }
                stdout.set_color(&default_color)?;
                writeln!(&mut stdout)?;
            }
            if !conf.filename_only && i < results.len() - 1 {
                println!();
            }
        }
    }
    Ok(!results.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color_mode::ColorMode;
    use pore_core::FileSearchResult;
    use std::path::PathBuf;

    #[test]
    fn snippet_segments_splits_on_highlight_tags() {
        let segments = snippet_segments("say <b>hello</b> now");
        assert_eq!(
            segments,
            vec![
                (false, "say ".to_string()),
                (true, "hello".to_string()),
                (false, " now".to_string()),
            ]
        );
    }

    #[test]
    fn snippet_segments_decodes_html_entities() {
        // Tantivy's Snippet::to_html escapes the fragment, so a literal `a & b <c>`
        // arrives as `a &amp; b &lt;c&gt;` and must not reach the terminal that way.
        let segments = snippet_segments("a &amp; b &lt;c&gt; &quot;d&quot; &#x27;e&#x27;");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].1, "a & b <c> \"d\" 'e'");
    }

    #[test]
    fn snippet_segments_emits_no_raw_ansi_escapes() {
        // Colour must go through termcolor so --color/piping is honoured.
        for (_, text) in snippet_segments("<b>hit</b> rest") {
            assert!(
                !text.contains('\x1b'),
                "segment carried a raw escape: {text}"
            );
        }
    }

    #[test]
    fn print_results_json_format() {
        let results = vec![FileSearchResult::with_snippets(
            PathBuf::from("test.txt"),
            0.5,
            vec!["<b>hello</b>".to_string()],
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
