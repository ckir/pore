### Task 1: pore-core LanguageRef unit tests

**Files:**
- Modify: `pore-core/src/language.rs` (append `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `LanguageRef` enum, `FromStr` impl, `Into<Language>` impl, `Serialize`/`Deserialize` impl, `mlua::FromLua` impl
- Produces: none

- [ ] **Step 1: Write tests for LanguageRef**

Append to `pore-core/src/language.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn all_variants_serialize_to_snake_case() {
        assert_eq!(serde_json::to_string(&LanguageRef::Arabic).unwrap(), "\"arabic\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Danish).unwrap(), "\"danish\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Dutch).unwrap(), "\"dutch\"");
        assert_eq!(serde_json::to_string(&LanguageRef::English).unwrap(), "\"english\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Finnish).unwrap(), "\"finnish\"");
        assert_eq!(serde_json::to_string(&LanguageRef::French).unwrap(), "\"french\"");
        assert_eq!(serde_json::to_string(&LanguageRef::German).unwrap(), "\"german\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Greek).unwrap(), "\"greek\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Hungarian).unwrap(), "\"hungarian\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Italian).unwrap(), "\"italian\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Norwegian).unwrap(), "\"norwegian\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Portuguese).unwrap(), "\"portuguese\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Romanian).unwrap(), "\"romanian\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Russian).unwrap(), "\"russian\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Spanish).unwrap(), "\"spanish\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Swedish).unwrap(), "\"swedish\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Tamil).unwrap(), "\"tamil\"");
        assert_eq!(serde_json::to_string(&LanguageRef::Turkish).unwrap(), "\"turkish\"");
    }

    #[test]
    fn all_variants_deserialize_from_snake_case() {
        assert_eq!(serde_json::from_str::<LanguageRef>("\"arabic\"").unwrap(), LanguageRef::Arabic);
        assert_eq!(serde_json::from_str::<LanguageRef>("\"english\"").unwrap(), LanguageRef::English);
        assert_eq!(serde_json::from_str::<LanguageRef>("\"turkish\"").unwrap(), LanguageRef::Turkish);
    }

    #[test]
    fn from_str_accepts_lowercase() {
        assert_eq!(LanguageRef::from_str("english").unwrap(), LanguageRef::English);
        assert_eq!(LanguageRef::from_str("arabic").unwrap(), LanguageRef::Arabic);
    }

    #[test]
    fn from_str_accepts_mixed_case() {
        assert_eq!(LanguageRef::from_str("English").unwrap(), LanguageRef::English);
        assert_eq!(LanguageRef::from_str("ENGLISH").unwrap(), LanguageRef::English);
        assert_eq!(LanguageRef::from_str("German").unwrap(), LanguageRef::German);
    }

    #[test]
    fn from_str_rejects_invalid() {
        assert!(LanguageRef::from_str("invalid").is_err());
        assert!(LanguageRef::from_str("").is_err());
    }

    #[test]
    fn lua_from_string_converts() {
        let lua = mlua::Lua::new();
        let val: mlua::Result<LanguageRef> = lua.load("'english'").eval();
        assert_eq!(val.unwrap(), LanguageRef::English);
    }

    #[test]
    fn lua_from_string_rejects_non_string() {
        let lua = mlua::Lua::new();
        let val: mlua::Result<LanguageRef> = lua.load("42").eval();
        assert!(val.is_err());
    }

    #[test]
    fn lua_from_string_rejects_invalid_language() {
        let lua = mlua::Lua::new();
        let val: mlua::Result<LanguageRef> = lua.load("'bogus'").eval();
        assert!(val.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p pore-core --features vendored language::tests -- --nocapture`
Expected: All 8 tests PASS

- [ ] **Step 3: Commit**

```bash
git add pore-core/src/language.rs
git commit -m "test: add LanguageRef unit tests"
```
