# Test Suite Design — pore

**Date:** 2026-08-08
**Goal:** Build an exhaustive test suite serving as both a safety net for future development and verification of correctness.
**Scope:** Deep coverage of critical search/indexing paths, representative tests for boilerplate. No property-based testing.

## Architecture

Tests follow Rust convention across three locations:

- **Inline `#[cfg(test)]` modules** — unit tests needing private access (single-function scope)
- **`tests/` integration directories** — end-to-end tests using only public APIs
- **`tests/common.rs`** — shared test utilities (temp dirs, fixture helpers, assertions)

### Test Distribution

| Crate | Share | Focus |
|---|---|---|
| pore-core | ~60% | FileIndex, GenericIndex, location, language, field_map, common |
| pore-bin | ~25% | args parsing, config loading, output formatting, CLI integration |
| pore-lua | ~15% | Lua userdata wrappers, options conversion, search from Lua |

## pore-core Tests

### FileIndex

| Category | Test Cases |
|---|---|
| **Create** | Empty directory; directory with files; with cache dir; without cache dir (in-memory); with custom FileIndexOptions |
| **Update** | No changes is no-op; new file added gets indexed; modified file re-indexed; rebuild flag forces full reindex |
| **Search** | Single term returns matches; multi-term returns combined results; no-match returns empty; threshold filters low scores; limit caps results; filename-only mode omits lines |
| **File walker** | Hidden files toggle; glob include; glob exclude (prefixed with !); glob case sensitivity; symlink following; .gitignore respect; oglob (only-search-matching) |
| **Delete** | Delete existing index returns true; delete nonexistent returns false; verify index files removed |
| **Metadata** | Serialize/deserialize round-trip; version tracks CARGO_PKG_VERSION; last-update timestamp advances on update; Display format includes key fields |
| **Options** | FileIndexOptionsShape merge (self none, other some → takes other); merge_from (other none → self unchanged); any() true when any Some; all() Ok when all Some; Lua table conversion with partial fields; Lua nil conversion succeeds |

### GenericIndex

| Category | Test Cases |
|---|---|
| **Add** | Single document with text fields; multiple documents in batch; missing field returns error |
| **Update** | Replace existing document by ID; fields fully replaced (not merged) |
| **Delete** | Delete by single ID; delete nonexistent ID is no-op; batch delete multiple IDs |
| **Search** | Basic query returns results; threshold filters; limit caps; empty index returns no results |
| **ID field** | Stored field used as document ID is findable; text fields are searchable but not stored |

### Location (position tracking)

| Category | Test Cases |
|---|---|
| **Position extraction** | Single term positions collected; multi-term positions collected; deleted documents skipped in position scan |
| **Line mapping** | Positions map to correct line numbers in multi-line file; empty file produces no lines; single-line file returns line 1; positions beyond file content handled gracefully |

### LanguageRef

| Category | Test Cases |
|---|---|
| **Serialization** | All 18 variants serialize to snake_case strings; all deserialize back from snake_case |
| **Parsing** | `from_str` accepts lowercase, mixed case, exact match; invalid string returns error |
| **Lua conversion** | String value converts to correct variant; non-string returns error; invalid string returns Lua error with message |

### FieldMap

| Category | Test Cases |
|---|---|
| **HashMap impl** | Get existing key returns value; get missing key returns error with field name |
| **Lua Table impl** | Get existing string field returns value; get missing field returns error; non-string value returns error |

### Common (metadata & index creation)

| Category | Test Cases |
|---|---|
| **Metadata** | `Metadata::new` sets version and epoch timestamp; set_last_update updates time; config() returns reference |
| **Index creation** | New index in ram with schema; new index on disk creates directory; corrupted index deleted and recreated; tokenizer registered with correct language stemmer |

## pore-bin Tests

### Args Parsing

| Category | Test Cases |
|---|---|
| **Minimal** | Query only → Search command, empty index/search options |
| **Index flags** | --hidden, --no-hidden conflict; --follow, --no-follow conflict; --language parses valid/invalid; --glob with comma-delimited values; --oglob with comma-delimited values; --glob-case-insensitive; --threads parses valid/invalid; --rebuild; --no-ignore |
| **Search flags** | --limit parses valid/invalid; --threshold parses valid/invalid; --json; --files-with-matches; --color never/auto/always/ansi |
| **Commands** | --files → ListFiles; --indexes → ListIndex; --delete → Delete; commands are mutually exclusive |
| **Overrides** | --in-memory / --no-memory; --update / --no-update |
| **--index** | With index name → uses named index, conflicts with other index flags |
| **Errors** | Invalid language → error; invalid threads → error; invalid limit → error; invalid threshold → error; invalid color → error |

### Config Loading

| Category | Test Cases |
|---|---|
| **Defaults** | No config file → default options |
| **Valid config** | Load full toml → all fields populated |
| **Partial config** | Load partial toml → unspecified fields are defaults |
| **Invalid config** | Malformed toml → error |
| **CLI merge** | CLI flags override config file values; config file values fill in missing CLI values |

### Output

| Category | Test Cases |
|---|---|
| **Text** | Results with matching lines formatted correctly; filename-only prints only paths; no results prints nothing |
| **JSON** | Results serialize to valid JSON; lines field omitted when empty; scores included |
| **Color** | auto detects terminal; always emits color codes; never emits plain text |

### CLI Integration (assert_cmd)

| Category | Test Cases |
|---|---|
| **Help** | `pore --help` exits 0, prints usage |
| **Search** | `pore "test" ./test-dir` exits 0, prints results |
| **Files** | `pore --files ./test-dir` exits 0, prints file list |
| **Indexes** | `pore --indexes` exits 0, prints index info |
| **Delete** | `pore --delete ./test-dir` exits 0 |
| **No args** | `pore` exits 0 (no query, no command) |

## pore-lua Tests

### FileIndexLua

| Category | Test Cases |
|---|---|
| **Create** | `pore.create_file_index(path)` creates or opens index |
| **Update** | `index:update()` indexes files; `index:update(true)` rebuilds |
| **Search** | `index:search("term")` returns table of results; `index:search("term", {limit=5})` respects options |
| **Delete** | `index:delete()` removes index |
| **ToString** | `tostring(index)` returns description string |

### GenericIndexLua

| Category | Test Cases |
|---|---|
| **Create** | `pore.create_generic_index(id, fields)` creates index |
| **Add** | `index:add_documents([{id="1", text="hello"}])` adds document |
| **Search** | `index:search("hello")` returns results with id and score |
| **Delete** | `index:delete_documents({"1"})` removes document |
| **ToString** | `tostring(index)` returns debug string |

### Options Conversion

| Category | Test Cases |
|---|---|
| **FileSearchOptionsShape** | Lua table `{limit=10, threshold=0.5}` → Rust struct; `{}` → defaults; `nil` → defaults; invalid field type → error |
| **SearchOptionsShape** | Same pattern for generic index options |
| **IndexOptionsShape** | `{language="english"}` → LanguageRef.English; invalid language → error |

## Test Infrastructure

### Dev Dependencies

- **pore-core:** `tempfile` (already in workspace via pore-bin dev-deps, add to pore-core)
- **pore-bin:** `assert_cmd = "2"`, `tempfile` (already present)
- **pore-lua:** no new deps (uses mlua test runtime)

### Fixture Strategy

All tests create content in `tempfile::tempdir()` (auto-cleanup on drop). Tests write small text files with known content for predictable search results. No static fixture files needed.

### Helper Utilities

Shared test helpers in `pore-core/tests/common.rs`:
- `create_test_files(dir, files)` — writes named files with given content
- `create_test_index(dir, options)` — creates FileIndex with temp cache
- `create_test_generic_index(id_field, text_fields, options)` — creates GenericIndex
- `search_and_collect(index, query_str, opts)` — parses query and runs search

Shared test helpers in `pore-bin/tests/common.rs`:
- `cmd()` — returns assert_cmd Command for pore binary
- `temp_config(content)` — writes temp toml config file

## Constraints

- Tests must pass on Windows (current dev platform)
- No reliance on external Lua installation (vendored Lua via feature flag for tests)
- Tests must be deterministic (no timing-dependent assertions, no random data)
- Each test isolates its filesystem operations (temp dirs only)
- `cargo test` from workspace root runs all tests
