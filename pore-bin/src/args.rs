use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};

use clap::ArgGroup;
use clap::{Arg, Command};
use pore_core::language::LanguageRef;
use pore_core::FileIndexOptionsShape;

use crate::color_mode::ColorMode;
use crate::config::SearchConfigOpt;

#[derive(Debug)]
pub enum CmdArg {
    Search,
    ListFiles,
    ListIndex,
    Delete,
}

#[derive(Debug)]
pub struct GlobalConfig {
    pub index: FileIndexOptionsShape,
    pub search: SearchConfigOpt,
    pub command: CmdArg,
    pub query: Option<String>,
    pub query_path: PathBuf,
    pub search_dir: String,
    pub index_name: Option<String>,
}

pub fn parse_args() -> Result<GlobalConfig, anyhow::Error> {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        // Index args
        .arg(
            Arg::new("index")
                .short('i')
                .long("index")
                .num_args(1)
                .conflicts_with_all(&["in_memory", "no_memory", "hidden", "no_hidden", "follow_links", "no_follow_links", "language", "glob", "oglob", "glob_case_insensitive"])
                .help("Use the specified index for querying (must be specified in the config file)")
        )
        .arg(
            Arg::new("update")
                .short('u')
                .long("update")
                .help("Update the index before searching (the default)"),
        )
        .arg(
            Arg::new("no_update")
                .long("no-update")
                .conflicts_with("update")
                .help("Do not update the index before performing the query"),
        )
        .arg(
            Arg::new("in_memory")
                .long("in-memory")
                .help("Do not store the text index on disk (will have to rebuild every time)"),
        )
        .arg(
            Arg::new("no_memory")
                .long("no-memory")
                .conflicts_with("in_memory")
                .help("Force the index to be saved to disk (overrides --in-memory)"),
        )
        .arg(
            Arg::new("hidden")
                .long("hidden")
                .help("Search hidden files and directories"),
        )
        .arg(
            Arg::new("no_hidden")
                .long("no-hidden")
                .conflicts_with("hidden")
                .help("Ignore hidden files and directories (overrides --hidden)"),
        )
        .arg(
            Arg::new("follow_links")
                .short('L')
                .long("follow")
                .help("Follow symbolic links"),
        )
        .arg(
            Arg::new("no_follow_links")
                .long("no-follow")
                .conflicts_with("follow_links")
                .help("Don't follow symbolic links (overrides --follow)"),
        )
        .arg(
            Arg::new("language")
                .long("language")
                .value_parser(LanguageRef::from_str)
                .help("The language to use for parsing files"),
        )
        .arg(
            Arg::new("glob")
                .short('g')
                .long("glob")
                .help("Include or exclude files and directories for searching that match the given glob. This always overrides any other ignore logic. Multiple glob flags may be used. Precede a glob with a ! to exclude it.")
                .value_delimiter(',')
                .num_args(1..)
        )
        .arg(
            Arg::new("oglob")
                .long("oglob")
                .help("Only search files that match this glob. Files that do not match any of these globs will be ignored.")
                .value_delimiter(',')
                .num_args(1..)
        )
        .arg(
            Arg::new("glob_case_insensitive")
                .long("glob-case-insensitive")
                .help("Patterns passed to --glob and --oglob will be matched in a case-insentive way.")
        )
        // Index args that don't conflict with --index
        .arg(
            Arg::new("threads")
                .short('j')
                .long("threads")
                .num_args(1)
                .value_parser(|a: &str| a.parse::<usize>().map_err(|_|"threads must be an unsigned integer".to_string()))
                .help("The approximate number of threads to use. A value of 0 (which is the default) will choose the thread count using heuristics.")
        )
        .arg(
            Arg::new("rebuild_index")
            .long("rebuild")
            .help("Force rebuild the index before searching")
        )

        // Search args
        .arg(
            Arg::new("limit")
                .long("limit")
                .num_args(1)
                .value_parser(|a: &str| a.parse::<usize>().map_err(|_|"limit must be an unsigned integer".to_string()))
                .help("Maximum number of files to return"),
        )
        .arg(
            Arg::new("threshold")
                .long("threshold")
                .num_args(1)
                .value_parser(|a: &str| a.parse::<f32>().map_err(|_|"threshold must be a floating point number".to_string()))
                .help("Minimum score threshold for results"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .conflicts_with("commands")
                .help("Print the results as json"),
        )
        .arg(
            Arg::new("files_with_matches")
                .short('l')
                .long("files-with-matches")
                .conflicts_with("commands")
                .help("Print out the files that match the search (not the matching lines)."),
        )
        .arg(
            Arg::new("no_ignore")
                .long("no-ignore")
                .help("Don't respect .gitignore files"),
        )
        .arg(
            Arg::new("color")
                .long("color")
                .num_args(1)
                .value_parser(["never", "auto", "always", "ansi"])
                .hide_possible_values(true)
                .help("This flag controls when to use colors. The default setting is auto, which will try to guess when to use colors.")
                .long_help("This flag controls when to use colors. The default setting is auto, which will try to guess when to use colors.
   The possible values for this flag are:

       never    Colors will never be used.
       auto     Auto-detect if the terminal supports colors (default).
       always   Colors will always be used regardless of where output is sent.
       ansi     Like 'always', but emits ANSI escapes (even in a Windows console).")
        )
        .group(
            ArgGroup::new("commands")
             .args(&["files", "indexes", "delete"])
            )
        .arg(
            Arg::new("files")
                .long("files")
                .help("Print out the files that would be searched (do not perform the search)"),
        )
        .arg(
            Arg::new("indexes")
                .long("indexes")
                .help("print out the indexes that would be used (do not perform the search)")
        )
        .arg(
            Arg::new("delete")
                .long("delete")
                .help("Delete the cached index files for the directory (if any)")
        )
        .arg(Arg::new("query"))
        .arg(Arg::new("dir"))
        .get_matches();

    let mut index = FileIndexOptionsShape::default();
    // Parse index options
    if matches.contains_id("hidden") && matches.get_flag("hidden") {
        index.hidden = Some(true);
    } else if matches.contains_id("no_hidden") && matches.get_flag("no_hidden") {
        index.hidden = Some(false);
    }
    if let Some(lang) = matches.get_one::<LanguageRef>("language") {
        index.language = Some(*lang);
    }
    if matches.contains_id("follow_links") && matches.get_flag("follow_links") {
        index.follow = Some(true);
    } else if matches.contains_id("no_follow_links") && matches.get_flag("no_follow_links") {
        index.follow = Some(false);
    }
    if matches.contains_id("no_ignore") && matches.get_flag("no_ignore") {
        index.ignore_files = Some(false);
    }
    if matches.contains_id("glob_case_insensitive") && matches.get_flag("glob_case_insensitive") {
        index.glob_case_insensitive = Some(true);
    }
    if let Some(globs) = matches.get_many::<String>("glob") {
        index.glob = Some(globs.map(|s| s.to_string()).collect());
    }
    if let Some(globs) = matches.get_many::<String>("oglob") {
        index.oglob = Some(globs.map(|s| s.to_string()).collect());
    }
    if let Some(threads) = matches.get_one::<usize>("threads") {
        index.threads = Some(*threads);
    }

    // Parse search options
    let mut search = SearchConfigOpt::default();
    if matches.contains_id("json") && matches.get_flag("json") {
        search.json = Some(true);
    }
    if let Some(limit) = matches.get_one::<usize>("limit") {
        search.limit = Some(*limit);
    }
    if let Some(threshold) = matches.get_one::<f32>("threshold") {
        search.threshold = Some(*threshold);
    }
    if matches.contains_id("files_with_matches") && matches.get_flag("files_with_matches") {
        search.filename_only = Some(true);
    }
    if let Some(color) = matches.get_one::<String>("color") {
        search.color = Some(ColorMode::from_str(color).unwrap());
    }
    if matches.contains_id("rebuild_index") && matches.get_flag("rebuild_index") {
        search.rebuild_index = Some(true);
    }
    if matches.contains_id("no_update") && matches.get_flag("no_update") {
        search.update = Some(false);
    } else if matches.contains_id("update") && matches.get_flag("update") {
        search.update = Some(true);
    };
    if matches.contains_id("in_memory") && matches.get_flag("in_memory") {
        search.in_memory = Some(true);
    } else if matches.contains_id("no_memory") && matches.get_flag("no_memory") {
        search.in_memory = Some(false);
    }

    let mut command = CmdArg::Search;
    if matches.contains_id("delete") && matches.get_flag("delete") {
        command = CmdArg::Delete;
    } else if matches.contains_id("files") && matches.get_flag("files") {
        command = CmdArg::ListFiles;
    } else if matches.contains_id("indexes") && matches.get_flag("indexes") {
        command = CmdArg::ListIndex;
    }
    let search_dir = matches.get_one::<String>("dir").cloned().unwrap_or_default();
    let query_path = if search_dir.is_empty() {
        env::current_dir()?
    } else {
        fs::canonicalize(Path::new(&search_dir))?
    };

    return Ok(GlobalConfig {
        index,
        search,
        command,
        query: matches.get_one::<String>("query").cloned(),
        query_path,
        search_dir,
        index_name: matches.get_one::<String>("index").cloned(),
    });
}
