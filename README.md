# Pore

[![Build Status](https://github.com/ckir/pore/actions/workflows/ci.yml/badge.svg)](https://github.com/ckir/pore/actions/workflows/ci.yml)
> pore (verb) \
> to read or study attentively

Pore is a command line [full-text
search](https://en.wikipedia.org/wiki/Full-text_search) tool powered by
[tantivy](https://github.com/quickwit-inc/tantivy).

**When would I use this instead of grep or ripgrep?**

If you can express what you're looking for as a regular expression or exact text
string, use ripgrep. If you want something more like a Google search, use pore.

**New in Pore:**
Pore now supports native regular expressions and field grouping in your queries!
- Regex searches: `pore search "contents:/w.lf/"`
- Regex over paths: `pore search "filepath:/.*\.rs/"`
- Field grouping: `pore search "contents:(big AND bad)"`
- Wildcards: `pore search "*foo"`

A regex must name a field — Tantivy rejects a bare `/.../` with *"Regex query need
to target a specific field"*. The two searchable fields are `contents` and `filepath`.
Note that `contents` is tokenized, so a regex there matches a **single term**:
`contents:/w.lf/` finds `wolf`, but a pattern containing a space can never match.
`filepath` is stored whole, so `filepath:/.*\.rs/` matches against the entire path.

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
      --snippets               Show Tantivy-generated snippets instead of matching lines
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
pore search "hello world"

# Exact phrase search
pore search '"exact phrase"'

# Boolean logic and grouping
pore search "hello AND (world OR universe)"

# Regex search (must target a field; matches one term in `contents`)
pore search "contents:/w.lf/"

# Field-specific search (e.g. searching only rust files)
pore search 'filepath:/.*\.rs/ AND foo'

# Sort results by modification date or file path
pore search "error" --sort date
pore search "error" --sort path

# Show condensed snippets with the match highlighted, instead of full matching lines
pore search "error" --snippets

# Aggregate analytics: see how many results exist per file extension
pore search "todo" --aggregate ext

# Aggregation output is JSON, so it composes with --jq
pore search "todo" --aggregate ext --jq '[.ext.buckets[].key]'

# Search hidden files and directories
pore search "secret" --hidden
```

## Lua module

`pore-lua` builds a native Lua module, so a Lua script can index and query directly
instead of shelling out to the CLI and parsing its output.

### Building

The default features build a cdylib with Lua **linked in**, which is not loadable as a
Lua module. You need the `module` feature, and it is mutually exclusive with the default
`vendored`:

```bash
cargo build -p pore-lua --release --no-default-features --features lua55,module
```

Swap `lua55` for `lua54`, `lua53`, `lua52`, `lua51`, or `luajit` to match your interpreter.

The artifact is `pore_lua.dll` on Windows, and `libpore_lua.so` / `libpore_lua.dylib`
elsewhere. Lua's loader looks for the module name without the `lib` prefix, so on
Unix rename or symlink it to `pore_lua.so` and put it on your `package.cpath`.

### Indexing files

```lua
local pore = require("pore_lua")

-- (directory to index, cache directory or nil for in-memory, options)
local idx = pore.get_file_index("/path/to/project", "/path/to/cache", {
  hidden = false,
  language = "english",
  glob = { "*.rs" },
})

idx:update(false)   -- true forces a full rebuild

for _, hit in ipairs(idx:search("contents:/w.lf/", { limit = 10 })) do
  print(hit.file, hit.score)
  for _, line in ipairs(hit.lines) do
    print(line.number, line.text)
  end
end
```

A hit is `{ file, score, lines }`, where each line is `{ number, text }` and `number` is
1-based. Passing `{ snippets = true }` swaps `lines` for `snippets`, a list of strings —
only one of the two is ever present. `{ aggregate = "ext" }` is available through
`idx:aggregate(query, opts)`, which returns the bucket counts as a table.

Search options: `limit`, `threshold`, `filename_only`, `root_dir`, `sort`, `snippets`,
`aggregate`. Index options: `follow`, `glob`, `oglob`, `glob_case_insensitive`, `hidden`,
`ignore_files`, `language`, `threads`. Omitted keys take their defaults.

### Indexing your own documents

`get_index` takes an id field, the text fields to index, options, and a cache directory:

```lua
local idx = pore.get_index("id", { "text" }, { language = "english" }, "/path/to/cache")

idx:add_documents({ { id = "1", text = "the big bad wolf" } })

for _, hit in ipairs(idx:search("wolf", { limit = 10 })) do
  print(hit.id, hit.score)      -- generic hits carry id and score only
end
```

Also available: `update_documents(docs)`, `delete_documents(ids)`, `delete()`.

Query syntax is the same as the CLI's, regexes included. `pore.version` holds
`full`, `major`, `minor`, `patch` and `pre`.

## Config
The config file is located at `${XDG_CONFIG_HOME}/pore.toml` (default
`$HOME/.config/pore.toml`). An example can be found at
[pore.example.toml](https://github.com/ckir/pore/blob/master/pore-bin/pore.example.toml).
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
