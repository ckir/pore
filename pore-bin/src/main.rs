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
        Commands::Search(args) => run_search(*args),
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
                let query_parser = QueryParser::for_index(index.index(), vec![*index.contents()]);
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
