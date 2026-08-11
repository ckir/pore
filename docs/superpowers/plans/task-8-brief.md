### Task 8: pore-bin CLI integration tests

**Files:**
- Modify: `pore-bin/Cargo.toml` — add `assert_cmd` dev-dependency
- Create: `pore-bin/tests/cli_integration.rs`

**Interfaces:**
- Consumes: pore binary at target
- Produces: none

- [ ] **Step 1: Add assert_cmd dev-dependency**

Add to `pore-bin/Cargo.toml` under `[dev-dependencies]`:

```toml
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 2: Create CLI integration tests**

Create `pore-bin/tests/cli_integration.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn pore() -> Command {
    Command::cargo_bin("pore").unwrap()
}

#[test]
fn help_exits_zero() {
    pore().arg("--help").assert().success();
}

#[test]
fn no_args_exits_zero() {
    pore().assert().success();
}

#[test]
fn files_command_lists_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    pore()
        .arg("--files")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn indexes_command_prints_index_info() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    // First create an index
    pore()
        .arg("test")
        .arg(tmp.path())
        .assert()
        .success();

    // Then list indexes
    pore()
        .arg("--indexes")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Index("));
}

#[test]
fn delete_command_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    // Create index first
    pore()
        .arg("test")
        .arg(tmp.path())
        .assert()
        .success();

    // Then delete
    pore()
        .arg("--delete")
        .arg(tmp.path())
        .assert()
        .success();
}

#[test]
fn search_command_finds_matches() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world from pore").unwrap();

    pore()
        .arg("pore")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn json_output_flag() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    pore()
        .arg("hello")
        .arg("--json")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"file\""));
}

#[test]
fn filename_only_flag() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world\nline two\nline three").unwrap();

    pore()
        .arg("hello")
        .arg("-l")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p pore --test cli_integration -- --nocapture`
Expected: All 8 tests PASS

- [ ] **Step 4: Commit**

```bash
git add pore-bin/Cargo.toml pore-bin/tests/cli_integration.rs
git commit -m "test: add CLI integration tests with assert_cmd"
```
