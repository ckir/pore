use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn pore() -> Command {
    Command::cargo_bin("pore").unwrap()
}

/// Returns a Command with `HOME` set to a temp directory.
/// On Windows, `HOME` is not set by default, and pore requires it for
/// config file lookup (`$HOME/.config/pore.toml`).
fn pore_with_home() -> (Command, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = pore();
    cmd.env("HOME", tmp.path());
    (cmd, tmp)
}

#[test]
fn help_exits_zero() {
    pore().arg("--help").assert().success();
}

#[test]
fn no_args_exits_zero() {
    // With no query, pore returns Ok(true) → exit 0.
    // HOME is needed for config file lookup and index path resolution.
    let (mut cmd, _tmp) = pore_with_home();
    cmd.assert().success();
}

#[test]
fn files_command_lists_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    // --files lists files that would be searched. Use --in-memory to avoid
    // the disk index path bug on Windows (strip_prefix("/") on absolute paths).
    // Positional args: [query] [dir] — use empty query to target a specific dir.
    let (mut cmd, _home) = pore_with_home();
    cmd.arg("--in-memory")
        .arg("--files")
        .arg("")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn indexes_command_prints_index_info() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    // --indexes prints index metadata. Use --in-memory to avoid Windows path issues.
    let (mut cmd, _home) = pore_with_home();
    cmd.arg("--in-memory")
        .arg("--indexes")
        .arg("")
        .arg(tmp.path())
        .assert()
        .success();
}

#[test]
fn delete_command_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    // --delete removes cached index files. With --in-memory this is a no-op but should still succeed.
    let (mut cmd, _home) = pore_with_home();
    cmd.arg("--in-memory")
        .arg("--delete")
        .arg("")
        .arg(tmp.path())
        .assert()
        .success();
}

#[test]
fn search_command_finds_matches() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world from pore").unwrap();

    // --rebuild forces index creation. Use --in-memory to avoid Windows path issues.
    let (mut cmd, _home) = pore_with_home();
    cmd.arg("--in-memory")
        .arg("--rebuild")
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

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("--in-memory")
        .arg("--rebuild")
        .arg("--json")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"file\""));
}

#[test]
fn filename_only_flag() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("test.txt"),
        "hello world\nline two\nline three",
    )
    .unwrap();

    // -l / --files-with-matches prints filenames only, not matching lines
    let (mut cmd, _home) = pore_with_home();
    cmd.arg("--in-memory")
        .arg("--rebuild")
        .arg("-l")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}
