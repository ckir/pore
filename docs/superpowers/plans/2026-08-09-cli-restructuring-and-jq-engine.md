# CLI Restructuring & jq Engine (SP-5) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a shared jq evaluation engine to `pore-core`, migrate the CLI from flat arguments to clap subcommands (`search`, `eval`), and wire the `--jq` post-processing flag and `pore eval` subcommand.

**Architecture:** A new `pore-core/src/jq.rs` module wraps `jaq-core`/`jaq-std`/`jaq-json` behind a `JqEngine` API. `pore-bin` is restructured from manual `clap` builder calls to `clap::Parser` derive macros with a `Commands` enum. `--jq` post-processes search JSON output; `pore eval` reads stdin/files and applies a jq filter.

**Tech Stack:** Rust, clap 4 (derive), jaq-core 3.1.0, jaq-std 3.1.0, jaq-json 3.1.0, tantivy 0.26.1

---

### Task 1: Add jaq Dependencies and Create `JqEngine`

**Files:**
- Modify: `pore-core/Cargo.toml`
- Create: `pore-core/src/jq.rs`
- Modify: `pore-core/src/lib.rs`

- [ ] **Step 1: Add jaq dependencies to `pore-core/Cargo.toml`**
Add these lines to the `[dependencies]` section:

```toml
jaq-core = "3.1.0"
jaq-std = "3.1.0"
jaq-json = "3.1.0"
```

- [ ] **Step 2: Create `pore-core/src/jq.rs` with the `JqEngine` struct**

```rust
//! jq filter compilation and evaluation engine.
//!
//! Wraps [`jaq_core`] to provide a simple compile-then-run API for jq expressions.
//! All jq integration in pore (CLI `--jq`, `pore eval`, Lua `jq()`, config formatters)
//! goes through [`JqEngine`].

use jaq_core::{Compiler, Ctx, RcIter};
use jaq_json::Val;

/// A compiled jq filter that can be run against JSON values.
pub struct JqEngine {
    filter: jaq_core::Filter<jaq_json::Native>,
}

impl JqEngine {
    /// Compile a jq filter string.
    ///
    /// Loads the `jaq-std` standard library (providing `map`, `select`,
    /// `group_by`, `sort_by`, etc.) before compilation.
    ///
    /// # Errors
    ///
    /// Returns an error if the filter string has invalid jq syntax.
    pub fn compile(filter_str: &str) -> Result<Self, anyhow::Error> {
        let mut defs = jaq_core::Definitions::core();
        let std_defs = jaq_std::std();
        defs.extend(std_defs);

        let (main, errs) = jaq_core::parse::parse(filter_str, jaq_core::parse::main());
        if !errs.is_empty() {
            return Err(anyhow::anyhow!(
                "jq parse error: {}",
                errs.iter()
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let main = main.ok_or_else(|| anyhow::anyhow!("jq parse produced no output"))?;

        let filter = Compiler::default()
            .with_funs(defs)
            .compile(main)
            .map_err(|errs| {
                anyhow::anyhow!(
                    "jq compile error: {}",
                    errs.iter()
                        .map(|e| format!("{e:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        Ok(Self { filter })
    }

    /// Run the compiled filter against a JSON value.
    ///
    /// Returns all output values produced by the filter. jq filters can produce
    /// zero, one, or many outputs from a single input.
    ///
    /// # Errors
    ///
    /// Returns an error if the filter encounters a runtime error (e.g.,
    /// accessing `.foo` on a non-object value).
    pub fn run(&self, input: &serde_json::Value) -> Result<Vec<serde_json::Value>, anyhow::Error> {
        let input_val: Val = Val::from(input.clone());
        let inputs = RcIter::new(core::iter::empty());
        let ctx = Ctx::new([], &inputs);

        let mut results = Vec::new();
        for output in self.filter.run((ctx, input_val)) {
            match output {
                Ok(val) => {
                    let json_val: serde_json::Value = val.into();
                    results.push(json_val);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("jq runtime error: {e:?}"));
                }
            }
        }
        Ok(results)
    }
}
```

- [ ] **Step 3: Register the `jq` module in `pore-core/src/lib.rs`**
Add `pub mod jq;` after the existing module declarations:

```rust
mod common;
mod field_map;
mod file;
mod generic;
pub mod jq;
pub mod language;
mod location;
```

- [ ] **Step 4: Run check**
Run: `cargo check -p pore-core --features vendored`
Expected: PASS. If jaq's API differs slightly from the code above (e.g., different method names in v3.1.0), fix by checking `jaq_core` docs with `cargo doc -p jaq-core --open`.

- [ ] **Step 5: Commit**
```bash
git add pore-core/Cargo.toml pore-core/src/jq.rs pore-core/src/lib.rs
git commit -m "feat: add JqEngine wrapping jaq-core for jq filter compilation and evaluation"
```

---

### Task 2: Unit Tests for `JqEngine`

**Files:**
- Modify: `pore-core/src/jq.rs`

- [ ] **Step 1: Add tests module to `pore-core/src/jq.rs`**
Append to the bottom of `pore-core/src/jq.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compile_valid_filter() {
        let engine = JqEngine::compile(".foo");
        assert!(engine.is_ok());
    }

    #[test]
    fn compile_invalid_filter() {
        let engine = JqEngine::compile(".[invalid syntax!!");
        assert!(engine.is_err());
    }

    #[test]
    fn run_identity_filter() {
        let engine = JqEngine::compile(".").unwrap();
        let input = json!({"a": 1, "b": 2});
        let results = engine.run(&input).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!({"a": 1, "b": 2}));
    }

    #[test]
    fn run_field_access() {
        let engine = JqEngine::compile(".name").unwrap();
        let input = json!({"name": "pore", "version": "0.2.0"});
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!("pore")]);
    }

    #[test]
    fn run_array_filter() {
        let engine = JqEngine::compile("[.[] | select(. > 2)]").unwrap();
        let input = json!([1, 2, 3, 4, 5]);
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!([3, 4, 5])]);
    }

    #[test]
    fn run_multiple_outputs() {
        let engine = JqEngine::compile(".[]").unwrap();
        let input = json!([10, 20, 30]);
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!(10), json!(20), json!(30)]);
    }

    #[test]
    fn run_empty_output() {
        let engine = JqEngine::compile("empty").unwrap();
        let input = json!(null);
        let results = engine.run(&input).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn run_string_interpolation() {
        let engine = JqEngine::compile(r#""\(.a):\(.b)""#).unwrap();
        let input = json!({"a": "hello", "b": "world"});
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!("hello:world")]);
    }

    #[test]
    fn run_sort_by() {
        let engine = JqEngine::compile("sort_by(.x)").unwrap();
        let input = json!([{"x": 3}, {"x": 1}, {"x": 2}]);
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!([{"x": 1}, {"x": 2}, {"x": 3}])]);
    }
}
```

- [ ] **Step 2: Run tests**
Run: `cargo test -p pore-core --features vendored -- jq`
Expected: All 8 tests PASS. If any fail due to jaq API differences, fix the `JqEngine` implementation to match the actual jaq v3.1.0 API.

- [ ] **Step 3: Commit**
```bash
git add pore-core/src/jq.rs
git commit -m "test: add unit tests for JqEngine"
```

---

### Task 3: Migrate CLI to Clap Derive Subcommands

**Files:**
- Modify: `pore-bin/Cargo.toml`
- Rewrite: `pore-bin/src/args.rs`
- Modify: `pore-bin/src/main.rs`

- [ ] **Step 1: Update `pore-bin/Cargo.toml` to use clap derive**
Change the `clap` dependency to include the `derive` feature:

```toml
clap = { version = "4.6.6", features = ["derive"] }
```

- [ ] **Step 2: Rewrite `pore-bin/src/args.rs`**
Replace the entire file with the new derive-based argument parsing. The key structural change: a `Cli` struct with a `Commands` enum containing `Search` and `Eval` subcommands.

```rust
//! CLI argument parsing for the `pore` binary.
//!
//! Uses clap derive macros to define a subcommand-based CLI:
//! - `pore search` — full-text search with indexing
//! - `pore eval` — evaluate a jq filter on JSON input

use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs};

use clap::{Args, Parser, Subcommand};
use pore_core::language::LanguageRef;
use pore_core::FileIndexOptionsShape;

use crate::color_mode::ColorMode;
use crate::config::SearchConfigOpt;

/// pore — full-text search powered by Tantivy
#[derive(Parser, Debug)]
#[command(name = "pore", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Full-text search a directory
    Search(SearchArgs),
    /// Evaluate a jq filter on JSON input (reads stdin or files)
    Eval(EvalArgs),
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// The search query
    pub query: Option<String>,

    /// The directory to search in
    pub dir: Option<String>,

    // --- Index options ---
    /// Use the specified index for querying (must be specified in the config file)
    #[arg(short = 'i', long = "index")]
    pub index_name: Option<String>,

    /// Update the index before searching (the default)
    #[arg(short = 'u', long)]
    pub update: bool,

    /// Do not update the index before performing the query
    #[arg(long, conflicts_with = "update")]
    pub no_update: bool,

    /// Do not store the text index on disk (will have to rebuild every time)
    #[arg(long)]
    pub in_memory: bool,

    /// Force the index to be saved to disk (overrides --in-memory)
    #[arg(long, conflicts_with = "in_memory")]
    pub no_memory: bool,

    /// Search hidden files and directories
    #[arg(long)]
    pub hidden: bool,

    /// Ignore hidden files and directories (overrides --hidden)
    #[arg(long, conflicts_with = "hidden")]
    pub no_hidden: bool,

    /// Follow symbolic links
    #[arg(short = 'L', long = "follow")]
    pub follow_links: bool,

    /// Don't follow symbolic links (overrides --follow)
    #[arg(long = "no-follow", conflicts_with = "follow_links")]
    pub no_follow_links: bool,

    /// The language to use for parsing files
    #[arg(long)]
    pub language: Option<String>,

    /// Include or exclude files and directories for searching that match the given glob
    #[arg(short = 'g', long, value_delimiter = ',', num_args = 1..)]
    pub glob: Option<Vec<String>>,

    /// Only search files that match this glob
    #[arg(long, value_delimiter = ',', num_args = 1..)]
    pub oglob: Option<Vec<String>>,

    /// Patterns passed to --glob and --oglob will be matched case-insensitively
    #[arg(long)]
    pub glob_case_insensitive: bool,

    /// The approximate number of threads to use (0 = auto)
    #[arg(short = 'j', long)]
    pub threads: Option<usize>,

    /// Force rebuild the index before searching
    #[arg(long)]
    pub rebuild: bool,

    // --- Search options ---
    /// Maximum number of files to return
    #[arg(long)]
    pub limit: Option<usize>,

    /// Minimum score threshold for results
    #[arg(long)]
    pub threshold: Option<f32>,

    /// Print the results as JSON
    #[arg(long)]
    pub json: bool,

    /// Print out the files that match the search (not the matching lines)
    #[arg(short = 'l', long)]
    pub files_with_matches: bool,

    /// Don't respect .gitignore files
    #[arg(long)]
    pub no_ignore: bool,

    /// Controls when to use colors (never, auto, always, ansi)
    #[arg(long, value_parser = ["never", "auto", "always", "ansi"])]
    pub color: Option<String>,

    /// Sort results by field (date, path). Defaults to relevance score.
    #[arg(short = 's', long)]
    pub sort: Option<String>,

    /// Aggregate results by field (e.g. ext)
    #[arg(long)]
    pub aggregate: Option<String>,

    // --- jq options ---
    /// Post-process the JSON output with a jq filter expression
    #[arg(long = "jq")]
    pub jq_expr: Option<String>,

    // --- Action flags ---
    /// Print out the files that would be searched (do not perform the search)
    #[arg(long)]
    pub files: bool,

    /// Print out the indexes that would be used (do not perform the search)
    #[arg(long)]
    pub indexes: bool,

    /// Delete the cached index files for the directory (if any)
    #[arg(long)]
    pub delete: bool,
}

#[derive(Args, Debug)]
pub struct EvalArgs {
    /// The jq filter expression to evaluate
    pub filter: String,

    /// Input files (reads stdin if none provided)
    pub files: Vec<PathBuf>,
}

/// The command to execute after configuration is resolved (used internally).
#[derive(Debug)]
pub enum CmdArg {
    Search,
    ListFiles,
    ListIndex,
    Delete,
}

/// Fully-resolved configuration after parsing CLI arguments.
#[derive(Debug)]
pub struct GlobalConfig {
    pub index: FileIndexOptionsShape,
    pub search: SearchConfigOpt,
    pub command: CmdArg,
    pub query: Option<String>,
    pub query_path: PathBuf,
    pub search_dir: String,
    pub index_name: Option<String>,
    pub jq_expr: Option<String>,
}

/// Build a [`GlobalConfig`] from parsed [`SearchArgs`].
pub fn build_search_config(args: SearchArgs) -> Result<GlobalConfig, anyhow::Error> {
    let mut index = FileIndexOptionsShape::default();

    if args.hidden {
        index.hidden = Some(true);
    } else if args.no_hidden {
        index.hidden = Some(false);
    }
    if let Some(ref lang) = args.language {
        index.language = Some(LanguageRef::from_str(lang)?);
    }
    if args.follow_links {
        index.follow = Some(true);
    } else if args.no_follow_links {
        index.follow = Some(false);
    }
    if args.no_ignore {
        index.ignore_files = Some(false);
    }
    if args.glob_case_insensitive {
        index.glob_case_insensitive = Some(true);
    }
    if let Some(globs) = args.glob {
        index.glob = Some(globs);
    }
    if let Some(globs) = args.oglob {
        index.oglob = Some(globs);
    }
    if let Some(threads) = args.threads {
        index.threads = Some(threads);
    }

    let mut search = SearchConfigOpt::default();
    if args.json {
        search.json = Some(true);
    }
    if let Some(limit) = args.limit {
        search.limit = Some(limit);
    }
    if let Some(threshold) = args.threshold {
        search.threshold = Some(threshold);
    }
    if args.files_with_matches {
        search.filename_only = Some(true);
    }
    if let Some(ref color) = args.color {
        search.color = Some(ColorMode::from_str(color).unwrap());
    }
    if args.rebuild {
        search.rebuild_index = Some(true);
    }
    if args.no_update {
        search.update = Some(false);
    } else if args.update {
        search.update = Some(true);
    }
    if args.in_memory {
        search.in_memory = Some(true);
    } else if args.no_memory {
        search.in_memory = Some(false);
    }

    let command = if args.delete {
        CmdArg::Delete
    } else if args.files {
        CmdArg::ListFiles
    } else if args.indexes {
        CmdArg::ListIndex
    } else {
        CmdArg::Search
    };

    let search_dir = args.dir.clone().unwrap_or_default();
    let query_path = if search_dir.is_empty() {
        env::current_dir()?
    } else {
        fs::canonicalize(&search_dir)?
    };

    Ok(GlobalConfig {
        index,
        search,
        command,
        query: args.query,
        query_path,
        search_dir,
        index_name: args.index_name,
        jq_expr: args.jq_expr,
    })
}
```

- [ ] **Step 3: Update `pore-bin/src/main.rs`**
Replace the contents with the new subcommand dispatch:

```rust
//! The `pore` binary — a command-line interface for indexing and searching code files.
//!
//! Supports two subcommands:
//! - `pore search` — full-text search with indexing
//! - `pore eval` — evaluate a jq filter on JSON input

#[macro_use]
extern crate anyhow;

use args::{build_search_config, Cli, CmdArg, Commands};
use clap::Parser;
use config::load_config;
use config::SearchConfig;
use pore_core::jq::JqEngine;
use pore_core::FileIndex;
use pore_core::FileIndexOptions;
use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process;
use tantivy::query::QueryParser;

mod args;
mod color_mode;
mod config;
mod output;

fn main() {
    match run() {
        Err(err) => {
            eprintln!("Error: {}", err);
            eprintln!("{:?}", err.backtrace());
            process::exit(2);
        }
        Ok(false) => {
            process::exit(1);
        }
        _ => {}
    }
}

fn run() -> Result<bool, anyhow::Error> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Search(args) => run_search(args),
        Commands::Eval(args) => run_eval(args),
    }
}

fn run_search(args: args::SearchArgs) -> Result<bool, anyhow::Error> {
    let conf = build_search_config(args)?;
    let (mut index_opt, mut search_opt) =
        load_config(&conf.query_path, conf.index_name.as_deref())?;
    search_opt.merge_from(&conf.search);
    if conf.index_name.is_some() {
        if conf.index.any() {
            bail!("Cannot use those arguments with --index");
        }
    } else {
        index_opt.merge_from(&conf.index);
    }
    let index: FileIndexOptions = index_opt.into();
    let search: SearchConfig = search_opt.into();

    let cache_dir = if search.in_memory {
        None
    } else {
        Some(find_index_dir(
            &conf.query_path,
            conf.index_name.as_deref(),
        )?)
    };
    let mut index = FileIndex::get_or_create(conf.query_path, cache_dir, &index)?;

    match conf.command {
        CmdArg::Delete => {
            index.delete()?;
            Ok(true)
        }
        CmdArg::ListFiles => {
            let walker = index.get_file_walker()?;
            for entry in walker.build().flatten() {
                println!("{}", entry.path().to_string_lossy());
            }
            Ok(true)
        }
        CmdArg::ListIndex => {
            println!("{}", index);
            Ok(true)
        }
        CmdArg::Search => {
            if search.update || search.rebuild_index {
                index.update(search.rebuild_index)?;
            }
            if let Some(query) = conf.query {
                let query_parser =
                    QueryParser::for_index(index.index(), vec![*index.contents()]);
                let query = query_parser.parse_query(&query)?;
                let opts = &search.to_opts(&conf.search_dir);
                let results = index.search(&query, opts)?;

                // If --jq is specified, post-process results as JSON
                if let Some(ref jq_expr) = conf.jq_expr {
                    let engine = JqEngine::compile(jq_expr)?;
                    let json_results = serde_json::to_value(&results)?;
                    let outputs = engine.run(&json_results)?;
                    let stdout = io::stdout();
                    let mut out = stdout.lock();
                    for output in outputs {
                        writeln!(out, "{}", serde_json::to_string_pretty(&output)?)?;
                    }
                    Ok(!results.is_empty())
                } else {
                    output::print_results(results, &search)
                }
            } else {
                Ok(true)
            }
        }
    }
}

fn run_eval(args: args::EvalArgs) -> Result<bool, anyhow::Error> {
    let engine = JqEngine::compile(&args.filter)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_output = false;

    if args.files.is_empty() {
        // Read from stdin
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            let input: serde_json::Value = serde_json::from_str(&line)?;
            let outputs = engine.run(&input)?;
            for output in outputs {
                writeln!(out, "{}", serde_json::to_string(&output)?)?;
                had_output = true;
            }
        }
    } else {
        // Read from files
        for file in &args.files {
            let contents = std::fs::read_to_string(file)?;
            let input: serde_json::Value = serde_json::from_str(&contents)?;
            let outputs = engine.run(&input)?;
            for output in outputs {
                writeln!(out, "{}", serde_json::to_string(&output)?)?;
                had_output = true;
            }
        }
    }

    Ok(had_output)
}

fn find_index_dir(for_dir: &Path, index_name: Option<&str>) -> Result<PathBuf, anyhow::Error> {
    let mut cache_home = env::var("XDG_CACHE_HOME").unwrap_or("".to_string());
    if cache_home.is_empty() {
        cache_home = env::var("HOME")? + "/.cache";
    }
    let mut index_root = PathBuf::from(cache_home);
    index_root.push(env!("CARGO_PKG_NAME"));
    if for_dir.is_absolute() {
        index_root.push(strip_root(for_dir));
    } else {
        index_root.push(strip_root(&env::current_dir()?));
        index_root.push(for_dir)
    }
    if let Some(name) = index_name {
        index_root.push(format!("__index_{}", name));
    }
    Ok(index_root)
}

fn strip_root(path: &Path) -> PathBuf {
    #[cfg(windows)]
    let skip: usize = 2;
    #[cfg(not(windows))]
    let skip: usize = 1;
    path.components().skip(skip).collect()
}
```

- [ ] **Step 4: Run check**
Run: `cargo check --workspace --features vendored`
Expected: PASS. Fix any compilation errors (e.g., import paths, trait bounds).

- [ ] **Step 5: Run existing tests**
Run: `cargo test --workspace --features vendored`
Expected: All existing tests PASS. If `output.rs` tests fail due to changed imports, fix them.

- [ ] **Step 6: Commit**
```bash
git add pore-bin/Cargo.toml pore-bin/src/args.rs pore-bin/src/main.rs
git commit -m "refactor!: migrate CLI to clap derive subcommands (search, eval)"
```

---

### Task 4: Integration Tests for `--jq` and `pore eval`

**Files:**
- Modify: `pore-bin/tests/` (create integration test files if they don't exist)

- [ ] **Step 1: Check if integration test infrastructure exists**
Run: `ls pore-bin/tests/` or check `pore-bin/Cargo.toml` for `[[test]]` sections.
If no integration test directory exists, create `pore-bin/tests/integration.rs`.

- [ ] **Step 2: Add integration test for `pore search --jq`**
Create or append to `pore-bin/tests/integration.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

#[test]
fn search_with_jq_filter() {
    let tmp = TempDir::new().unwrap();
    let test_file = tmp.path().join("hello.txt");
    fs::write(&test_file, "hello world from pore").unwrap();

    let output = Command::cargo_bin("pore")
        .unwrap()
        .args(["search", "hello", "--jq", "[.[].file]", "--in-memory"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello.txt"), "Expected filename in jq output: {}", stdout);
}

#[test]
fn search_with_jq_invalid_filter_errors() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("test.txt"), "test content").unwrap();

    Command::cargo_bin("pore")
        .unwrap()
        .args(["search", "test", "--jq", ".[invalid!!", "--in-memory"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}
```

- [ ] **Step 3: Add integration test for `pore eval`**
Append to `pore-bin/tests/integration.rs`:

```rust
#[test]
fn eval_from_file() {
    let tmp = TempDir::new().unwrap();
    let json_file = tmp.path().join("data.json");
    fs::write(&json_file, r#"{"name":"pore","version":"0.2.0"}"#).unwrap();

    let output = Command::cargo_bin("pore")
        .unwrap()
        .args(["eval", ".name"])
        .arg(&json_file)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pore"), "Expected 'pore' in eval output: {}", stdout);
}

#[test]
fn eval_invalid_filter_errors() {
    Command::cargo_bin("pore")
        .unwrap()
        .args(["eval", ".[bad syntax!!"])
        .assert()
        .failure();
}
```

- [ ] **Step 4: Run integration tests**
Run: `cargo test --features vendored -p pore`
Expected: All tests PASS.

- [ ] **Step 5: Commit**
```bash
git add pore-bin/tests/
git commit -m "test: add integration tests for --jq flag and pore eval subcommand"
```

---

### Task 5: Update README and Help Text

**Files:**
- Modify: `README.md`
- Modify: `pore-bin/src/args.rs` (help text is already in derive attrs from Task 3)

- [ ] **Step 1: Update the Examples section in `README.md`**
Add jq examples after the existing examples:

```markdown
# Post-process search results with jq
pore search "error" --jq '[.[].file]'
pore search "TODO" --jq '[.[] | select(.score > 5)]'

# Evaluate a jq filter on a JSON file
echo '{"a":1,"b":2}' | pore eval '.a + .b'
pore eval '[.[] | select(.status == "active")]' data.json
```

Update the existing examples to use the `search` subcommand:

```markdown
# Basic Google-like search for files containing both words
pore search "hello world"

# Exact phrase search
pore search '"exact phrase"'
```

- [ ] **Step 2: Regenerate the Usage section**
Run: `python .github/update_readme.py`
Expected: The Usage section in README.md is updated to reflect the new subcommand structure.

- [ ] **Step 3: Run final test suite**
Run: `cargo test --workspace --features vendored`
Expected: All tests PASS.

- [ ] **Step 4: Commit**
```bash
git add README.md
git commit -m "docs: update README for subcommand CLI and jq examples"
```
