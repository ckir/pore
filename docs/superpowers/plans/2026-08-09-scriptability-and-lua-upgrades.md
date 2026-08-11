# Scriptability & Lua Upgrades Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose `pore`'s new sorting, aggregation, and JSON indexing features to Lua scripts via `pore-lua`.

**Architecture:** Update `mlua` interoperability shapes in `pore-core` to deserialize the new fields. Add the `aggregate` method to Lua's `FileIndex` wrapper and convert Tantivy's JSON output to Lua tables using `mlua::Lua::to_value`.

**Tech Stack:** Rust, mlua, tantivy 0.26.1

---

### Task 1: Update Lua API Shapes in `pore-core`

**Files:**
- Modify: `pore-core/src/lib.rs`
- Modify: `pore-core/src/file.rs`

- [ ] **Step 1: Update `FileSearchOptionsShape`**
In `pore-core/src/lib.rs`, add `pub sort: Option<String>` and `pub aggregate: Option<String>` to `FileSearchOptionsShape` (with `#[serde(default)]` and `#[serde(skip_serializing_if = "Option::is_none")]` annotations to match existing fields if applicable). 

- [ ] **Step 2: Update `Into<FileSearchOptions>`**
In `pore-core/src/file.rs`, update the `impl From<FileSearchOptionsShape> for FileSearchOptions` to correctly map the new `sort` and `aggregate` fields from the shape struct into the final options struct.

- [ ] **Step 3: Run tests**
Run: `cargo check -p pore-core --features vendored`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add pore-core/src/lib.rs pore-core/src/file.rs
git commit -m "feat: expose sort and aggregate fields to Lua options shape"
```

---

### Task 2: Implement Lua Bindings in `pore-lua`

**Files:**
- Modify: `pore-lua/src/lib.rs`

- [ ] **Step 1: Update `get_index` in `pore_lua`**
Modify the `get_index` closure in `pore-lua/src/lib.rs` (around line 75). Change its signature to optionally accept a fifth parameter for `add_json_field: Option<bool>`.
```rust
        |_,
         (id_field, text_fields, config, cache_dir, add_json_field): (
            String,
            Vec<String>,
            IndexOptionsShape,
            Option<String>,
            Option<bool>, // NEW
        )| {
```
Pass `add_json_field.unwrap_or(false)` as the final parameter to `GenericIndex::get_or_create`.

- [ ] **Step 2: Add `:aggregate()` to `FileIndexLua`**
Add an `aggregate` method to `FileIndexLua` in `pore-lua/src/lib.rs`.
```rust
        methods.add_method(
            "aggregate",
            |lua, this, (query_str, opts): (String, FileSearchOptionsShape)| {
                let query_parser = QueryParser::for_index(this.index.index(), vec![*this.index.contents()]);
                let query = query_parser
                    .parse_query(&query_str)
                    .map_err(|_| LuaError::RuntimeError("Error parsing query".to_string()))?;
                
                let json_val = this
                    .index
                    .aggregate(&query, &opts.into())
                    .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                
                lua.to_value(&json_val).map_err(|e| LuaError::RuntimeError(e.to_string()))
            },
        );
```

- [ ] **Step 3: Update Tests**
Add unit tests to `pore-lua/src/lib.rs` verifying that `sort` and `aggregate` can be parsed from a Lua string. 
Modify the test `file_search_options_shape_from_lua_table` to include `sort = "date"` and `aggregate = "ext"`, and assert they decode correctly.
Add a test verifying `get_index` can take 5 arguments (you can invoke it via a mock `Lua` instance or update the existing generic index tests if applicable).

- [ ] **Step 4: Run tests**
Run: `cargo test -p pore-lua --features vendored`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add pore-lua/src/lib.rs
git commit -m "feat: expose aggregations and JSON indexing to Lua scripts"
```
