### Task 7: pore-bin color_mode and output unit tests

**Files:**
- Modify: `pore-bin/src/color_mode.rs` (append `#[cfg(test)]` module)
- Modify: `pore-bin/src/output.rs` (append `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `ColorMode`, `FromStr` impl, `print_results`
- Produces: none

- [ ] **Step 1: Write ColorMode tests**

Append to `pore-bin/src/color_mode.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn color_mode_from_str_valid() {
        assert_eq!(ColorMode::from_str("auto").unwrap(), ColorMode::Auto);
        assert_eq!(ColorMode::from_str("always").unwrap(), ColorMode::Always);
        assert_eq!(ColorMode::from_str("ansi").unwrap(), ColorMode::Ansi);
        assert_eq!(ColorMode::from_str("never").unwrap(), ColorMode::Never);
    }

    #[test]
    fn color_mode_from_str_case_insensitive() {
        assert_eq!(ColorMode::from_str("AUTO").unwrap(), ColorMode::Auto);
        assert_eq!(ColorMode::from_str("Always").unwrap(), ColorMode::Always);
    }

    #[test]
    fn color_mode_from_str_invalid() {
        assert!(ColorMode::from_str("invalid").is_err());
    }

    #[test]
    fn color_mode_into_color_choice() {
        let choice: ColorChoice = ColorMode::Always.into();
        assert!(matches!(choice, ColorChoice::Always));

        let choice: ColorChoice = ColorMode::Ansi.into();
        assert!(matches!(choice, ColorChoice::AlwaysAnsi));

        let choice: ColorChoice = ColorMode::Never.into();
        assert!(matches!(choice, ColorChoice::Never));
    }
}
```

- [ ] **Step 2: Write output tests**

Append to `pore-bin/src/output.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pore_core::{FileSearchResult, Line};
    use std::path::PathBuf;

    #[test]
    fn print_results_json_format() {
        let results = vec![FileSearchResult {
            file: PathBuf::from("test.txt"),
            score: 0.5,
            lines: vec![Line { number: 1, text: "hello".to_string() }],
        }];
        let conf = SearchConfig {
            json: true,
            color: ColorMode::Never,
            ..SearchConfig::default()
        };
        let result = print_results(results, &conf);
        assert!(result.unwrap());
    }

    #[test]
    fn print_results_empty_returns_false() {
        let conf = SearchConfig::default();
        let result = print_results(vec![], &conf);
        assert!(!result.unwrap());
    }

    #[test]
    fn print_results_non_empty_returns_true() {
        let results = vec![FileSearchResult {
            file: PathBuf::from("test.txt"),
            score: 0.5,
            lines: vec![],
        }];
        let conf = SearchConfig::default();
        let result = print_results(results, &conf);
        assert!(result.unwrap());
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p pore-bin color_mode::tests output::tests -- --nocapture`
Expected: All 7 tests PASS

- [ ] **Step 4: Commit**

```bash
git add pore-bin/src/color_mode.rs pore-bin/src/output.rs
git commit -m "test: add ColorMode and output unit tests"
```
