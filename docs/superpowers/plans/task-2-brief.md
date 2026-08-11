### Task 2: pore-core FieldMap unit tests

**Files:**
- Modify: `pore-core/src/field_map.rs` (append `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `FieldMap` trait, `HashMap<String, String>` impl, `mlua::Table` impl
- Produces: none

- [ ] **Step 1: Write tests for FieldMap**

Append to `pore-core/src/field_map.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn hashmap_get_existing_field() {
        let mut map = HashMap::new();
        map.insert("title".to_string(), "Hello World".to_string());
        let result = map.get_field("title").unwrap();
        assert_eq!(result.as_ref(), "Hello World");
    }

    #[test]
    fn hashmap_get_missing_field_returns_error() {
        let map: HashMap<String, String> = HashMap::new();
        let result = map.get_field("missing");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing"));
    }

    #[test]
    fn lua_table_get_existing_field() {
        let lua = mlua::Lua::new();
        let tbl: mlua::Table = lua.load("{ name = 'test' }").eval().unwrap();
        let result = tbl.get_field("name").unwrap();
        assert_eq!(result.as_ref(), "test");
    }

    #[test]
    fn lua_table_get_missing_field_returns_error() {
        let lua = mlua::Lua::new();
        let tbl: mlua::Table = lua.load("{}").eval().unwrap();
        let result = tbl.get_field("missing");
        assert!(result.is_err());
    }

    #[test]
    fn lua_table_non_string_value_returns_error() {
        let lua = mlua::Lua::new();
        let tbl: mlua::Table = lua.load("{ count = 42 }").eval().unwrap();
        let result = tbl.get_field("count");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p pore-core --features vendored field_map::tests -- --nocapture`
Expected: All 5 tests PASS

- [ ] **Step 3: Commit**

```bash
git add pore-core/src/field_map.rs
git commit -m "test: add FieldMap unit tests"
```
