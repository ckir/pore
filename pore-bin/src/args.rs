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
