//! Abstraction for extracting named text fields from heterogeneous data sources.
//!
//! The [`FieldMap`] trait provides a uniform way to read string-valued fields
//! from different backing stores — currently [`HashMap<String, String>`] and
//! [`mlua::Table`]. This lets [`GenericIndex`](crate::generic::GenericIndex)
//! accept documents from either Rust code or Lua scripts without needing
//! separate indexing paths.

use std::{borrow::Cow, collections::HashMap};

/// Extracts a named text field from a data source.
///
/// Implementations return [`Cow::Borrowed`] when the backing store can provide
/// a reference (e.g., `HashMap`), and [`Cow::Owned`] when a conversion is
/// required (e.g., Lua table → Rust string).
///
/// # Errors
/// Returns an error if the field is missing or not a string.
pub trait FieldMap {
    /// Returns the value of the field with the given key.
    fn get_field(&self, key: &str) -> anyhow::Result<Cow<'_, str>>;
}

impl FieldMap for HashMap<String, String> {
    fn get_field(&self, key: &str) -> anyhow::Result<Cow<'_, str>> {
        self.get(key)
            .map(|s| Cow::Borrowed(s.as_str()))
            .ok_or_else(|| anyhow!("Missing field {}", key))
    }
}

impl FieldMap for mlua::Table {
    fn get_field(&self, key: &str) -> anyhow::Result<Cow<'_, str>> {
        self.get::<String>(key)
            .map(Cow::Owned)
            .map_err(|e| anyhow!("{}", e))
    }
}

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
        let tbl: mlua::Table = lua.load("{ nested = {} }").eval().unwrap();
        let result = tbl.get_field("nested");
        assert!(result.is_err());
    }
}
