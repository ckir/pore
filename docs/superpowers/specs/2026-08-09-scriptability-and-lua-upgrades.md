# Spec: Scriptability & Lua Upgrades (Sub-Project 4)

## Objective
Expose the new analytical, sorting, and structured data indexing superpowers developed in Sub-Projects 2 and 3 to `pore`'s Lua API (`pore-lua`). This ensures that scripts interacting with `pore` programmatically have feature parity with the CLI.

## Tech Stack
- **Rust** (1.75+)
- **mlua** (for Lua bindings)
- **tantivy 0.26.1**

## Commands
```bash
# Testing
cargo test -p pore-lua --features vendored
cargo test -p pore-core --features vendored

# Linting
cargo clippy -- -D warnings
```

## Project Structure
- `pore-core/src/lib.rs` - Extend `FileSearchOptionsShape` (used for `mlua::FromLua`) to include the new `sort: Option<String>` and `aggregate: Option<String>` fields.
- `pore-lua/src/lib.rs` - 
  - Update `get_index` to accept the new `add_json_field: bool` parameter (as an optional 5th parameter).
  - Add the `:aggregate(query, opts)` method to `FileIndexLua` userdata to expose aggregations.
  - Convert `serde_json::Value` (returned by `.aggregate()`) into `mlua::Value` so Lua can read the results natively as a table.

## Code Style
```rust
// Exposing new methods in mlua
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
            
        // Convert serde_json to mlua::Value for seamless Lua table access
        lua.to_value(&json_val)
    },
);
```

## Testing Strategy
- **Unit Tests:** 
  - Add tests in `pore-lua/src/lib.rs` confirming that a Lua table like `{ limit = 5, sort = "date", aggregate = "ext" }` correctly deserializes into `FileSearchOptionsShape`.
  - Add a test that calls `:aggregate()` on a mocked index and confirms that a Lua table of bucket counts is returned.
  - Update `generic_index_lua_add_search_delete` or add a new test to pass `add_json_field` to `get_index`.

## Boundaries
- **Always:** Use `lua.to_value(&json_value)` to convert JSON payloads into native Lua tables rather than returning stringified JSON.
- **Always:** Ensure any new parameters to existing Lua functions (like `get_index`) are optional so we do not break backwards compatibility with existing scripts.
- **Ask first:** If there is a need to refactor `pore-core` logic to accommodate Lua bindings. The bindings should wrap existing logic natively.
- **Never:** Drop support for older scripts calling `search` or `get_index`.

## Success Criteria
1. Lua scripts can invoke `get_index(..., ..., ..., true)` to create JSON-supported indexes.
2. Lua scripts can pass `{sort = "date"}` and `{aggregate = "ext"}` into `:search()` and `:aggregate()` methods respectively.
3. `:aggregate` returns a native Lua table corresponding to the underlying JSON aggregation bucket tree.
4. All tests in `pore-lua` compile and pass.

## Open Questions
- None at this time.
