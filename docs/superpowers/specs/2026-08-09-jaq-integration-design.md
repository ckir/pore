# Design: jaq Integration into pore

> **Status:** Approved  
> **Date:** 2026-08-09  
> **Scope:** Full roadmap — 4 sub-projects  
> **Breaking Change:** Yes — CLI migrates from flat args to subcommands  

## Objective

Integrate [jaq](https://github.com/01mf02/jaq) (a pure-Rust jq clone, MIT licensed) into pore to enable jq-based post-processing of search results, standalone JSON evaluation, config-driven output formatting, structured data querying, index-time transforms, and Lua scripting support.

## Background

pore v0.2.0 ships with `--sort`, `--aggregate`, `json_data` fields, snippet generation, and ANSI colors. The natural next step is to let users reshape, filter, and transform data using the jq language — a well-known, composable query language for JSON.

jaq is ideal because:
- `jaq-core` has **zero dependencies**
- `jaq-json` only needs `serde_json` (already in pore)
- Pure Rust, compiles into the binary, faster than jq on most benchmarks
- MIT licensed, same as pore

## Dependency Strategy

Always included (no feature gate). The dependency cost is effectively zero.

| Crate | Version | Purpose | New Transitive Deps |
|---|---|---|---|
| `jaq-core` | 3.1.0 | Parser, compiler, interpreter | 0 |
| `jaq-std` | 3.1.0 | Standard library (`map`, `select`, `group_by`, etc.) | 0 (only `jaq-core`) |
| `jaq-json` | 3.1.0 | JSON value type implementing `ValT` | 0 (only `serde_json`, already present) |

Added to `pore-core/Cargo.toml` only. `pore-bin` and `pore-lua` access jaq through the shared engine module.

## Architecture

### New Module: `pore-core/src/jq.rs`

A shared jq compilation and evaluation engine. All features go through this single module.

```rust
pub struct JqEngine { /* compiled filter */ }

impl JqEngine {
    /// Compile a jq filter string. Returns error if syntax is invalid.
    pub fn compile(filter: &str) -> Result<Self>;

    /// Run the compiled filter against a JSON value.
    /// Returns all output values (jq can produce multiple outputs).
    pub fn run(&self, input: &serde_json::Value) -> Result<Vec<serde_json::Value>>;
}
```

### File Layout Changes

```
pore-core/
  src/
    jq.rs              ← NEW: shared jq engine
    file.rs            (extended with transform option)
    generic.rs         (extended with transform option)
    lib.rs             (re-exports jq module)

pore-bin/
  src/
    main.rs            ← REWRITE: clap subcommands
    args.rs            ← REWRITE: subcommand-based arg parsing
    output.rs          (updated for --jq post-processing and --format)
    config.rs          (extended with [format.<name>] sections)

pore-lua/
  src/
    lib.rs             (extended with pore.jq() function)
```

### CLI Subcommand Structure (Post-Migration)

```
pore search "query" [dir] [--jq <filter>] [--jq-filter <filter>] [--sort ...] [--aggregate ...] [--format <name>] [--json] ...
pore eval <filter> [file...]
```

- `pore search` — all existing search functionality, plus new jq flags
- `pore eval` — standalone jq evaluator; reads stdin or files, runs filter, prints output
- Bare `pore "query"` **no longer works** (clean break, not backwards-compatible)

---

## Sub-Project Decomposition

### SP-5: CLI Restructuring & jq Engine (Foundation)

**Depends on:** Nothing  
**Breaking:** Yes (subcommand migration)

#### Part A: jq Engine

- New file `pore-core/src/jq.rs`
- `JqEngine::compile(filter)` — compiles via `jaq_core::Compiler` with `jaq_std` definitions loaded
- `JqEngine::run(input)` — evaluates against a `serde_json::Value`, returns `Vec<Value>`
- Compilation errors returned immediately on invalid filter syntax
- Runtime errors (e.g., `.foo` on a non-object) surfaced per-value
- Unit tests: compile valid/invalid filters, run against sample JSON, verify output

#### Part B: Subcommand Migration

- Rewrite `pore-bin/src/args.rs` to use `clap::Parser` derive macros with `#[derive(Subcommand)]`
- `Commands` enum: `Search(SearchArgs)`, `Eval(EvalArgs)`
- All existing flags (`--hidden`, `--sort`, `--aggregate`, `--limit`, etc.) move into `SearchArgs`
- `--files`, `--indexes`, `--delete` remain as flags within `SearchArgs` (search-context operations)
- `EvalArgs`: `filter: String`, `files: Vec<PathBuf>` (if empty, read stdin)
- `main.rs` rewritten to match on `Commands` enum

#### Part C: `--jq` and `eval` Implementation

- **`pore search --jq <filter>`:** After search returns results, serialize to `serde_json::Value`, run through `JqEngine`, print output. `--jq` implicitly enables JSON-mode output.
- **`pore eval <filter>`:** Read stdin (or files), parse as JSON (one value per line for streaming), run through `JqEngine`, print each output value.
- `--jq` and `--json` can coexist (jq post-processes JSON output)
- `--jq` and `--format` are mutually exclusive

#### Testing

- Unit tests for `JqEngine` in `pore-core`
- Integration tests for `pore search --jq` verifying end-to-end filter application
- Integration tests for `pore eval` verifying stdin/file reading and filter execution
- Regression tests ensuring all existing search flags work under `pore search`

---

### SP-6: Analytics & Formatting

**Depends on:** SP-5

#### Part A: jq for Aggregations

`--jq` already post-processes whatever JSON `search` produces. If `--aggregate` output flows through the same JSON→jq pipeline, this is zero additional code. Verify and add integration tests:

```bash
pore search "fn" --aggregate ext --jq '.buckets | sort_by(-.doc_count) | .[0:5]'
```

#### Part B: Config-Driven Output Formatters

New `[format.<name>]` section in `pore.toml`:

```toml
[format.compact]
jq = '"\(.file):\(.score)"'

[format.csv]
jq = '"\(.file),\(.score),\(.snippets | length)"'

[format.markdown]
jq = '"- [\(.file)](\(.file)) (score: \(.score))"'
```

- New CLI flag: `--format <name>`
- `config.rs` loads format definitions at startup
- `--format <name>` looks up the jq expression, compiles via `JqEngine`, applies per-result
- Output printed as raw strings (not JSON-wrapped) since the jq expression produces the formatted string
- `--format` and `--jq` are mutually exclusive

#### Testing

- Integration tests for `--aggregate` + `--jq` pipeline
- Unit tests for config parsing of `[format.*]` sections
- Integration tests for `--format` flag with sample `pore.toml`

---

### SP-7: Structured Data & Transforms

**Depends on:** SP-5

#### Part A: `json_data` Querying with `--jq-filter`

New CLI flag `--jq-filter <expr>` that runs **per-document** during result emission:

```bash
pore search "status:active" --jq-filter '.json_data | select(.priority == "high")'
```

Flow:
1. Tantivy returns matching documents
2. For each doc, deserialize `json_data` field into `serde_json::Value`
3. Run `--jq-filter` expression via `JqEngine`
4. If filter produces output → include document in results
5. If filter produces empty → skip document

Key distinction from `--jq`:
- `--jq` post-processes the **entire result set** (after search)
- `--jq-filter` filters **individual documents** (during search)
- They can be combined

Implementation:
- Add `jq_filter: Option<String>` to `FileSearchOptions` (flows through `create_option_copy` macro)
- In `FileIndex::search`, after retrieving each document, optionally run the jq filter
- Same for `GenericIndex::search`

#### Part B: Index-Time jq Transform Pipeline

New `transform` option on both `FileIndexOptions` and `IndexOptions`:

```rust
pub struct FileIndexOptions {
    // ... existing fields ...
    pub transform: Option<String>,  // jq expression applied before indexing
}
```

Behavior:
- **FileIndex:** After reading file contents, if contents parse as valid JSON, run the transform. If they don't parse as JSON, index as-is (plain text files unaffected).
- **GenericIndex:** Run transform on each document's field map (serialized as JSON object) before adding to index.

Configurable via `pore.toml`:
```toml
[local-myapi]
    path = "/data/api-dump"
    transform = '.body |= gsub("<[^>]+>"; "") | .title |= ascii_downcase'
```

The transform runs once at index time — no search performance cost.

#### Testing

- Unit tests for `--jq-filter` with mock documents containing `json_data`
- Integration tests for index-time transforms on both FileIndex and GenericIndex
- Edge case: non-JSON file contents with transform configured (should index as-is)

---

### SP-8: Lua jq Bindings

**Depends on:** SP-5 (jq engine), SP-7 (transform/jq_filter fields on options structs)

#### `pore.jq()` Function

New top-level function in `pore_lua` module:

```lua
local pore = require("pore")

-- jq on a Lua table
local results = idx:search("error", { limit = 100 })
local filenames = pore.jq(results, "[.[].file]")

-- jq on a raw JSON string
local parsed = pore.jq('{"a":1,"b":2}', '.a + .b')
```

Implementation:
- Accepts Lua table or JSON string as input
- If table: convert to `serde_json::Value` via `mlua::Lua::from_value`
- Run through `JqEngine`
- Convert output back to Lua table via `lua.to_value`
- Returns a Lua table of results (jq can produce multiple outputs)

#### Transform and jq_filter Wiring

Both `transform` (from SP-7) and `jq_filter` are fields on `FileIndexOptions`/`IndexOptions`/`FileSearchOptions`, which flow through `create_option_copy` into Lua shapes automatically:

```lua
local idx = pore.get_file_index("/data/logs", {
    transform = '.msg |= ascii_downcase'
})

local results = idx:search("error", {
    jq_filter = 'select(.severity == "critical")'
})
```

No additional Lua-specific wiring needed — the macro handles it.

#### Testing

- Unit test: `pore.jq()` with Lua table input and various filters
- Unit test: `pore.jq()` with JSON string input
- Unit test: `transform` option flows through to Lua shape
- Unit test: `jq_filter` option flows through to Lua shape

---

## Sub-Project Dependency Graph

```
SP-5 (Foundation: CLI + jq Engine)
 ├── SP-6 (Analytics & Formatting)
 ├── SP-7 (Structured Data & Transforms)
 └── SP-8 (Lua jq Bindings) ← also depends on SP-7 for transform fields
```

SP-6 and SP-7 are independent of each other and can be parallelized after SP-5. SP-8 depends on SP-7 for the `transform`/`jq_filter` fields but can start the `pore.jq()` function work after SP-5.

## Success Criteria

1. `pore search "query" --jq '<filter>'` post-processes results with jq expressions
2. `pore eval '<filter>'` works as a standalone jq evaluator on stdin/files
3. `pore search --format <name>` applies config-defined jq formatters
4. `pore search --jq-filter '<expr>'` filters individual documents by `json_data` content
5. Index-time transforms via `transform` option work for both FileIndex and GenericIndex
6. `pore.jq()` Lua function accepts tables/strings and returns native Lua tables
7. All existing functionality preserved under `pore search` subcommand
8. All tests pass across `pore-core`, `pore-bin`, and `pore-lua`
