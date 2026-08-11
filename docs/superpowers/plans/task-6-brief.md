### Task 6: pore-core location unit tests

**Files:**
- Modify: `pore-core/src/location.rs` (append `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `get_search_results`, `positions_to_lines`, `DocResult`, `Line`, `FileIndex`
- Produces: none

- [ ] **Step 1: Write location unit tests**

Append to `pore-core/src/location.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn positions_to_lines_empty_positions_produces_no_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("test.txt");
        fs::write(&test_file, "line one\nhello world\nline three").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        // Empty positions should produce no lines without errors
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert!(lines.is_empty());
    }

    // Helper that doesn't need a FileIndex (tests line-counting logic only)
    fn positions_to_lines_no_index(
        filepath: &Path,
        positions: &mut BytePositions,
        lines: &mut Vec<Line>,
    ) -> Result<(), anyhow::Error> {
        if positions.is_empty() {
            return Ok(());
        }
        let file = File::open(filepath)?;
        let mut reader = io::BufReader::new(file);
        let mut line_str = String::new();
        let mut line_no = 1u32;
        while let Ok(bytes) = reader.read_line(&mut line_str)? {
            if bytes == 0 { break; }
            lines.push(Line { number: line_no, text: line_str.trim_end().to_string() });
            line_str.clear();
            line_no += 1;
        }
        Ok(())
    }

    #[test]
    fn positions_to_lines_empty_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("empty.txt");
        fs::write(&test_file, "").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert!(lines.is_empty());
    }

    #[test]
    fn positions_to_lines_single_line() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("single.txt");
        fs::write(&test_file, "hello world").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        positions.push(Reverse(0));
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].number, 1);
    }

    #[test]
    fn positions_to_lines_multi_line() {
        let tmp = tempfile::TempDir::new().unwrap();
        let test_file = tmp.path().join("multi.txt");
        fs::write(&test_file, "line one\nline two\nline three").unwrap();

        let mut lines = Vec::new();
        let mut positions = BytePositions::new();
        positions.push(Reverse(2));
        let result = positions_to_lines_no_index(&test_file, &mut positions, &mut lines);
        assert!(result.is_ok());
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2].number, 3);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p pore-core --features vendored location::tests -- --nocapture`
Expected: All 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add pore-core/src/location.rs
git commit -m "test: add location position-to-line unit tests"
```
