# Pore

[![Build Status](https://github.com/stevearc/pore/actions/workflows/ci.yml/badge.svg)](https://github.com/stevearc/pore/actions)
[![Crates.io](https://img.shields.io/crates/v/pore.svg)](https://crates.io/crates/pore)
> pore (verb) \
> to read or study attentively

Pore is a command line [full-text
search](https://en.wikipedia.org/wiki/Full-text_search) tool powered by
[tantivy](https://github.com/quickwit-inc/tantivy).

**When would I use this instead of grep or ripgrep?**

If you can express what you're looking for as a regular expression or exact text
string, use ripgrep. If you want something more like a Google search, use pore.

```
Usage:

Commands:
  search  Full-text search a directory
  eval    Evaluate a jq filter on JSON input (reads stdin or files)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Examples:
  # Basic search
  pore search "hello world"
  pore search "fn main" src/

  # Search with glob include/exclude patterns
  pore search "TODO" -g '*.rs,*.toml'
  pore search "error" --oglob 'target/*,.git/*'

  # Search hidden files or follow symlinks
  pore search "secret" --hidden
  pore search "config" -L

  # Language-scoped search
  pore search "import" --language python

  # Force index rebuild, or skip update
  pore search "query" --rebuild
  pore search "query" --no-update

  # Output as JSON, with jq post-processing
  pore search "bug" --json
  pore search "bug" --json --jq '.files[] | .path'
  pore search "error" --jq '.results | length'

  # Sort, limit, and threshold
  pore search "panic" --sort path --limit 20
  pore search "leak" --threshold 0.5

  # List files that would be searched (dry run)
  pore search --files
  pore search --indexes

  # Delete cached index for the current directory
  pore search --delete

  # Evaluate a jq filter on JSON input
  echo '{"a": 1, "b": 2}' | pore eval '.a + .b'
  pore eval '.results[] | .path' results.json
  pore eval '.' file1.json file2.json

### `pore search`

Usage: pore search [OPTIONS] [QUERY] [DIR]

Arguments:
  [QUERY]  The search query
  [DIR]    The directory to search in

Options:
  -i, --index <INDEX_NAME>     Use the specified index for querying (must be specified in the config file)
  -u, --update                 Update the index before searching (the default)
      --no-update              Do not update the index before performing the query
      --in-memory              Do not store the text index on disk (will have to rebuild every time)
      --no-memory              Force the index to be saved to disk (overrides --in-memory)
      --hidden                 Search hidden files and directories
      --no-hidden              Ignore hidden files and directories (overrides --hidden)
  -L, --follow                 Follow symbolic links
      --no-follow              Don't follow symbolic links (overrides --follow)
      --language <LANGUAGE>    The language to use for parsing files
  -g, --glob <GLOB>...         Include or exclude files and directories for searching that match the given glob
      --oglob <OGLOB>...       Only search files that match this glob
      --glob-case-insensitive  Patterns passed to --glob and --oglob will be matched case-insensitively
  -j, --threads <THREADS>      The approximate number of threads to use (0 = auto)
      --rebuild                Force rebuild the index before searching
      --limit <LIMIT>          Maximum number of files to return
      --threshold <THRESHOLD>  Minimum score threshold for results
      --json                   Print the results as JSON
  -l, --files-with-matches     Print out the files that match the search (not the matching lines)
      --no-ignore              Don't respect .gitignore files
      --color <COLOR>          Controls when to use colors (never, auto, always, ansi) [possible values: never, auto, always, ansi]
  -s, --sort <SORT>            Sort results by field (date, path). Defaults to relevance score
      --aggregate <AGGREGATE>  Aggregate results by field (e.g. ext)
      --jq <JQ_EXPR>           Post-process the JSON output with a jq filter expression
      --files                  Print out the files that would be searched (do not perform the search)
      --indexes                Print out the indexes that would be used (do not perform the search)
      --delete                 Delete the cached index files for the directory (if any)
  -h, --help                   Print help

### `pore eval`

Usage: pore eval <FILTER> [FILES]...

Arguments:
  <FILTER>    The jq filter expression to evaluate
  [FILES]...  Input files (reads stdin if none provided)

Options:
  -h, --help  Print help
```

## Examples

```bash
# Basic Google-like search for files containing both words
pore "hello world"

# Exact phrase search
pore '"exact phrase"'

# Boolean logic and grouping
pore "hello AND (world OR universe)"

# Regex search
pore "/b.* wolf/"

# Field-specific search (e.g. searching only rust files)
pore "path:*.rs AND foo"

# Sort results by modification date or file path
pore "error" --sort date
pore "error" --sort path

# Aggregate analytics: see how many results exist per file extension
pore "todo" --aggregate ext

# Search hidden files and directories
pore "secret" --hidden
```

## Config
The config file is located at `${XDG_CONFIG_HOME}/pore.toml` (default
`$HOME/.config/pore.toml`). An example can be found at
[pore.example.toml](https://github.com/stevearc/pore/blob/master/pore-bin/pore.example.toml).
The format is:

```toml
# These are the global arguments that are used by default
limit = 10

# You can add an index with different customizations.
# These are used by passing --index=NAME
[index-NAME]
    oglob = "*.md,*.rst,*.txt"

# You can add additional customizations for a specific directory
[local-myproject]
    # Be sure to specify the path
    path = "/path/to/myproject"

    # These options will override the global ones
    language = "Arabic"

    # Local projects can specify their own indexes.
    # They are also used by passing --index=OTHER_INDEX
    [local-myproject.OTHER_INDEX]
        limit = 20
```
