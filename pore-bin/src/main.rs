//! The `pore` binary — a command-line interface for indexing and searching code files.
//!
//! `pore` is the primary user-facing entry point. It parses CLI arguments, loads optional
//! TOML configuration, creates or opens a Tantivy-backed file index, and executes one of
//! four commands: search, list-files, list-index, or delete.
//!
//! # Index cache location
//!
//! When an on-disk index is used, it is stored under `$XDG_CACHE_HOME/pore/<project-path>/`.
//! If `XDG_CACHE_HOME` is unset, it falls back to `$HOME/.cache`. Named indexes (via
//! `--index`) append a `__index_<name>` suffix to allow multiple indexes per directory.

#[macro_use]
extern crate anyhow;

use args::CmdArg;
use config::load_config;
use config::SearchConfig;
use pore_core::FileIndex;
use pore_core::FileIndexOptions;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use tantivy::query::QueryParser;

mod args;
mod color_mode;
mod config;
mod output;

fn main() {
    match run_cmd() {
        Err(err) => {
            eprintln!("Error: {}", err);
            eprintln!("{:?}", err.backtrace());
            process::exit(2);
        }
        Ok(false) => {
            // Command ran successfully but found no results.
            process::exit(1);
        }
        _ => {}
    }
}

/// Entry point that parses arguments, loads config, and dispatches the requested command.
///
/// Returns `Ok(true)` when results were found, `Ok(false)` when the command succeeded but
/// produced no results, and `Err` on failure.
fn run_cmd() -> Result<bool, anyhow::Error> {
    let conf = args::parse_args()?;
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
    let mut index = FileIndex::get_or_create(conf.query_path, cache_dir, &index.into())?;

    match conf.command {
        CmdArg::Delete => {
            index.delete()?;
            return Ok(true);
        }
        CmdArg::ListFiles => {
            let walker = index.get_file_walker()?;
            for result in walker.build() {
                if let Ok(entry) = result {
                    println!("{}", entry.path().to_string_lossy());
                }
            }
            return Ok(true);
        }
        CmdArg::ListIndex => {
            println!("{}", index);
            return Ok(true);
        }
        CmdArg::Search => {
            if search.update || search.rebuild_index {
                index.update(search.rebuild_index)?;
            }
            if let Some(query) = conf.query {
                let query_parser = QueryParser::for_index(&index.index(), vec![*index.contents()]);
                let query = query_parser.parse_query(&query)?;
                let opts = &search.to_opts(&conf.search_dir);
                let results = index.search(&query, &opts)?;
                return output::print_results(results, &search);
            } else {
                return Ok(true);
            }
        }
    }
}

/// Computes the on-disk cache path for the Tantivy index.
///
/// The path follows the XDG Base Directory convention:
/// `$XDG_CACHE_HOME/pore/<for_dir>/` (or `$HOME/.cache/pore/<for_dir>/`).
///
/// If `index_name` is provided, a `__index_<name>` segment is appended so that multiple
/// named indexes can coexist for the same directory.
fn find_index_dir(for_dir: &Path, index_name: Option<&str>) -> Result<PathBuf, anyhow::Error> {
    let mut cache_home = env::var("XDG_CACHE_HOME").unwrap_or("".to_string());
    if cache_home == "" {
        cache_home = env::var("HOME")? + "/.cache";
    }
    let mut index_root = PathBuf::from(cache_home);
    index_root.push(env!("CARGO_PKG_NAME"));
    if for_dir.is_absolute() {
        index_root.push(for_dir.strip_prefix("/")?);
    } else {
        index_root.push(env::current_dir()?.strip_prefix("/")?);
        index_root.push(for_dir)
    }
    if let Some(name) = index_name {
        index_root.push(format!("__index_{}", name));
    }
    return Ok(index_root);
}
