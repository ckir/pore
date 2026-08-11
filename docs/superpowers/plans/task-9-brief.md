### Task 9: pore-lua unit and integration tests

**Files:**
- Modify: `pore-lua/src/lib.rs` (append `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `FileIndexLua`, `GenericIndexLua`, option shapes, pore_core types
- Produces: none

- [ ] **Step 1: Write Lua binding tests**

Append to `pore-lua/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;
    use pore_core::{FileIndexOptions, FileSearchOptions, IndexOptions, SearchOptions};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn version_table_populated() {
        let lua = Lua::new();
        let tbl = make_version_tbl(&lua).unwrap();
        let major: String = tbl.get("major").unwrap();
        assert!(!major.is_empty());
    }

    #[test]
    fn file_index_lua_create_update_search() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.txt"), "hello world from pore").unwrap();

        let opts = FileIndexOptions::default();
        let index = FileIndex::get_or_create(tmp.path(), Some(tmp.path()), &opts).unwrap();

        // Test update
        index.clone().update(false).unwrap();

        // Test search
        let query_parser = tantivy::query::QueryParser::for_index(
            index.index(),
            vec![*index.contents()],
        );
        let query = query_parser.parse_query("pore").unwrap();
        let results = index.search(&query, &FileSearchOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn file_index_lua_tostring() {
        let tmp = TempDir::new().unwrap();
        let opts = FileIndexOptions::default();
        let index = FileIndex::get_or_create(tmp.path(), Some(tmp.path()), &opts).unwrap();
        let s = format!("{}", index);
        assert!(s.contains("Index("));
    }

    #[test]
    fn generic_index_lua_add_search_delete() {
        let tmp = TempDir::new().unwrap();
        let opts = IndexOptions::default();
        let mut index =
            GenericIndex::get_or_create("id", vec!["text"], &opts, Some(tmp.path())).unwrap();

        let mut doc = HashMap::new();
        doc.insert("id".to_string(), "1".to_string());
        doc.insert("text".to_string(), "hello world".to_string());
        index.add_documents(vec![doc]).unwrap();

        let query_parser =
            tantivy::query::QueryParser::for_index(index.index(), index.get_text_fields());
        let query = query_parser.parse_query("hello").unwrap();
        let results = index.search(&query, &SearchOptions::default()).unwrap();
        assert_eq!(results.len(), 1);

        index.delete_documents(vec!["1".to_string()]).unwrap();
        let results = index.search(&query, &SearchOptions::default()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn file_search_options_shape_from_lua_table() {
        let lua = Lua::new();
        let opts: FileSearchOptionsShape = lua
            .load("{ limit = 5, threshold = 0.3 }")
            .eval()
            .unwrap();
        assert_eq!(opts.limit, Some(5));
        assert_eq!(opts.threshold, Some(0.3));
        assert_eq!(opts.filename_only, None);
    }

    #[test]
    fn file_search_options_shape_from_lua_nil() {
        let lua = Lua::new();
        let opts: FileSearchOptionsShape = lua.load("nil").eval().unwrap();
        assert_eq!(opts.limit, None);
        assert_eq!(opts.threshold, None);
    }

    #[test]
    fn index_options_shape_from_lua_table() {
        let lua = Lua::new();
        let opts: IndexOptionsShape = lua.load("{ language = 'english' }").eval().unwrap();
        assert_eq!(opts.language, Some(pore_core::language::LanguageRef::English));
    }

    #[test]
    fn search_options_shape_from_lua_table() {
        let lua = Lua::new();
        let opts: SearchOptionsShape = lua.load("{ limit = 10 }").eval().unwrap();
        assert_eq!(opts.limit, Some(10));
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p pore-lua --features vendored -- --nocapture`
Expected: All 8 tests PASS

- [ ] **Step 3: Commit**

```bash
git add pore-lua/src/lib.rs
git commit -m "test: add Lua binding unit and integration tests"
```
