//! End-to-end coverage of the terminal rendering path.
//!
//! These spawn the real `pore` binary and assert on the raw bytes it writes to stdout.
//! That is the point: the rest of the suite checks `--json`, and the `print_results`
//! unit tests only assert the returned value, so nothing else in the repo would notice
//! if colour started leaking into piped output or if highlight markup reached the user
//! verbatim. Both have already happened once.
//!
//! Note on `--color auto`: `assert_cmd` captures stdout through a pipe, so stdout is
//! never a TTY here and `auto` always resolves to "no colour". That is exactly the case
//! worth pinning -- it is the one a user hits when piping into `less` or a file. The
//! TTY branch of `auto` would need a pty and is not covered.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

const ESC: u8 = 0x1b;

/// Runs a search against a fixture directory and returns raw stdout bytes.
///
/// `HOME` is redirected to a temp dir because pore resolves its config from
/// `$HOME/.config/pore.toml`, and on Windows `HOME` is not set by default.
fn search_stdout(files: &[(&str, &str)], extra_args: &[&str]) -> (TempDir, Vec<u8>) {
    let tmp = TempDir::new().unwrap();
    for (name, contents) in files {
        fs::write(tmp.path().join(name), contents).unwrap();
    }
    let home = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pore").unwrap();
    cmd.env("HOME", home.path())
        .arg("search")
        .arg("--in-memory")
        .arg("--rebuild");
    for a in extra_args {
        cmd.arg(a);
    }
    let out = cmd.arg("hello").arg(tmp.path()).output().unwrap();
    assert!(
        out.status.success(),
        "pore failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (tmp, out.stdout)
}

fn fixture() -> Vec<(&'static str, &'static str)> {
    vec![("f.txt", "alpha line\nhello match\ngamma line\n")]
}

fn esc_count(bytes: &[u8]) -> usize {
    bytes.iter().filter(|b| **b == ESC).count()
}

// --- no colour must mean no escape bytes at all -----------------------------------

#[test]
fn color_never_emits_no_escape_bytes() {
    let (_t, out) = search_stdout(&fixture(), &["--color", "never"]);
    assert!(!out.is_empty(), "expected some output to inspect");
    assert_eq!(
        esc_count(&out),
        0,
        "--color never must emit no ANSI escapes, got: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn color_auto_when_piped_emits_no_escape_bytes() {
    // Regression guard for 35db023: `auto` resolves to Never when stdout is not a TTY.
    let (_t, out) = search_stdout(&fixture(), &["--color", "auto"]);
    assert!(!out.is_empty(), "expected some output to inspect");
    assert_eq!(
        esc_count(&out),
        0,
        "--color auto piped must not colour, got: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn color_never_still_prints_the_match_and_line_number() {
    // Guards against "no escapes" being satisfied by printing nothing useful.
    let (_t, out) = search_stdout(&fixture(), &["--color", "never"]);
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("hello match"), "missing match text: {text}");
    assert!(text.contains("2:"), "missing 1-based line number: {text}");
}

// --- forced colour must actually colour --------------------------------------------

#[test]
fn color_ansi_emits_escape_bytes() {
    let (_t, out) = search_stdout(&fixture(), &["--color", "ansi"]);
    assert!(
        esc_count(&out) > 0,
        "--color ansi must emit ANSI escapes, got: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn color_always_emits_escape_bytes() {
    let (_t, out) = search_stdout(&fixture(), &["--color", "always"]);
    assert!(
        esc_count(&out) > 0,
        "--color always must emit colour, got: {:?}",
        String::from_utf8_lossy(&out)
    );
}

// --- snippets go through termcolor, not hardcoded escapes ---------------------------

#[test]
fn snippets_with_color_never_emit_no_escape_bytes() {
    // Regression guard: the snippet renderer once wrote "\x1b[31m" directly, which
    // ignored --color entirely and leaked escapes into piped and redirected output.
    let (_t, out) = search_stdout(&fixture(), &["--snippets", "--color", "never"]);
    assert!(!out.is_empty(), "expected some output to inspect");
    assert_eq!(
        esc_count(&out),
        0,
        "--snippets must honour --color never, got: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn snippets_with_color_auto_piped_emit_no_escape_bytes() {
    let (_t, out) = search_stdout(&fixture(), &["--snippets", "--color", "auto"]);
    assert_eq!(
        esc_count(&out),
        0,
        "--snippets piped must not colour, got: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn snippets_with_color_ansi_emit_escape_bytes() {
    let (_t, out) = search_stdout(&fixture(), &["--snippets", "--color", "ansi"]);
    assert!(
        esc_count(&out) > 0,
        "--snippets --color ansi must highlight, got: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn snippets_never_leak_html_markup_to_the_terminal() {
    // Tantivy's Snippet::to_html wraps matches in <b> tags and escapes the fragment.
    // Neither may reach the user in either colour mode.
    for mode in ["never", "ansi"] {
        let (_t, out) = search_stdout(&fixture(), &["--snippets", "--color", mode]);
        let text = String::from_utf8_lossy(&out);
        assert!(
            !text.contains("<b>") && !text.contains("</b>"),
            "--color {mode}: highlight tags leaked verbatim: {text}"
        );
        assert!(
            !text.contains("&amp;") && !text.contains("&lt;") && !text.contains("&gt;"),
            "--color {mode}: HTML entities leaked verbatim: {text}"
        );
    }
}

// --- machine-readable output is never coloured --------------------------------------

#[test]
fn json_output_is_never_colored_even_with_color_always() {
    // --json is consumed by other programs (including pore's own --jq), so colour must
    // never reach it regardless of the colour flag.
    let (_t, out) = search_stdout(&fixture(), &["--json", "--color", "always"]);
    assert!(!out.is_empty(), "expected some output to inspect");
    assert_eq!(
        esc_count(&out),
        0,
        "--json must never be coloured, got: {:?}",
        String::from_utf8_lossy(&out)
    );
    // And it must still be parseable.
    let text = String::from_utf8_lossy(&out);
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|e| panic!("--json line is not valid JSON ({e}): {line}"));
    }
}
