### Task 3: pore-core common/metadata unit tests

**Files:**
- Modify: `pore-core/src/common.rs` (append `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `Metadata`, `IndexMetadata` trait, `MetadataConfig` trait, `create_index`, `delete_index`, `METADATA_FILE`
- Produces: none

- [ ] **Step 1: Write tests for Metadata and index creation**

Append to `pore-core/src/common.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct TestConfig {
        language: LanguageRef,
    }
    impl MetadataConfig for TestConfig {
        fn language(&self) -> LanguageRef {
            self.language
        }
    }

    #[test]
    fn metadata_new_sets_version_and_epoch() {
        let config = TestConfig { language: LanguageRef::English };
        let meta = Metadata::<TestConfig>::new(config.clone());
        assert_eq!(meta.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(meta.config().language, LanguageRef::English);
    }

    #[test]
    fn metadata_set_last_update() {
        let config = TestConfig { language: LanguageRef::English };
        let mut meta = Metadata::<TestConfig>::new(config);
        let now = Utc::now();
        meta.set_last_update(now);
        assert_eq!(meta.last_update(), &now);
    }

    #[test]
    fn create_index_in_ram() {
        let config = TestConfig { language: LanguageRef::English };
        let (meta_opt, index) = create_index::<Metadata<TestConfig>, _, _, Vec<String>>(
            None::<&Path>,
            &config,
            "id",
            vec!["text".to_string()],
        )
        .unwrap();
        assert!(meta_opt.is_none());
        assert!(index.schema().get_field("id").is_ok());
        assert!(index.schema().get_field("text").is_ok());
    }

    #[test]
    fn create_index_on_disk() {
        let tmp = TempDir::new().unwrap();
        let config = TestConfig { language: LanguageRef::English };
        let (meta_opt, index) = create_index::<Metadata<TestConfig>, _, _, Vec<String>>(
            Some(tmp.path()),
            &config,
            "id",
            vec!["text".to_string()],
        )
        .unwrap();
        assert!(meta_opt.is_none());
        // Verify metadata file created
        assert!(tmp.path().join(METADATA_FILE).exists());
    }

    #[test]
    fn create_index_reloads_existing_metadata() {
        let tmp = TempDir::new().unwrap();
        let config = TestConfig { language: LanguageRef::English };
        // First create
        create_index::<Metadata<TestConfig>, _, _, Vec<String>>(
            Some(tmp.path()),
            &config,
            "id",
            vec!["text".to_string()],
        )
        .unwrap();
        // Second create should load existing meta
        let (meta_opt, _) = create_index::<Metadata<TestConfig>, _, _, Vec<String>>(
            Some(tmp.path()),
            &config,
            "id",
            vec!["text".to_string()],
        )
        .unwrap();
        assert!(meta_opt.is_some());
    }

    #[test]
    fn delete_index_returns_false_for_in_memory() {
        let config = TestConfig { language: LanguageRef::English };
        let (_, index) = create_index::<Metadata<TestConfig>, _, _, Vec<String>>(
            None::<&Path>,
            &config,
            "id",
            vec!["text".to_string()],
        )
        .unwrap();
        let result = delete_index(&index, None).unwrap();
        assert!(!result);
    }

    #[test]
    fn delete_index_on_disk_removes_files() {
        let tmp = TempDir::new().unwrap();
        let config = TestConfig { language: LanguageRef::English };
        let (_, index) = create_index::<Metadata<TestConfig>, _, _, Vec<String>>(
            Some(tmp.path()),
            &config,
            "id",
            vec!["text".to_string()],
        )
        .unwrap();
        let result = delete_index(&index, Some(tmp.path())).unwrap();
        assert!(result);
        assert!(!tmp.path().exists());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p pore-core --features vendored common::tests -- --nocapture`
Expected: All 7 tests PASS

- [ ] **Step 3: Commit**

```bash
git add pore-core/src/common.rs
git commit -m "test: add metadata and index creation unit tests"
```
