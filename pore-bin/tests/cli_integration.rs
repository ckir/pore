use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn pore() -> Command {
    Command::cargo_bin("pore").unwrap()
}

/// Returns a Command with `HOME` pointed at a temp directory.
///
/// pore no longer *requires* `HOME` -- it falls back to `USERPROFILE` -- but these
/// tests still redirect it so they read a config file that does not exist rather than
/// whatever the developer happens to have at `~/.config/pore.toml`. Tests that must
/// prove the fallback works remove the variable instead; see `works_with_home_unset`.
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
fn no_args_prints_help_and_fails() {
    let (mut cmd, _tmp) = pore_with_home();
    cmd.assert().failure().code(2);
}

#[test]
fn files_command_lists_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
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

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
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

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
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

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
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
    cmd.arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--json")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"file\""));
}

#[test]
fn json_output_defaults_to_lines_not_snippets() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--json")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"lines\""))
        .stdout(predicate::str::contains("\"snippets\"").not());
}

#[test]
fn snippets_flag_switches_json_to_snippets() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--json")
        .arg("--snippets")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"snippets\""))
        .stdout(predicate::str::contains("\"lines\"").not());
}

#[test]
fn filename_only_flag() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("test.txt"),
        "hello world\nline two\nline three",
    )
    .unwrap();

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("-l")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn search_with_jq_filter() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("hello.txt"), "hello world from pore").unwrap();

    let (mut cmd, _home) = pore_with_home();
    let output = cmd
        .arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--jq")
        .arg("[.[].file]")
        .arg("hello")
        .arg(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello.txt"),
        "Expected filename in jq output: {}",
        stdout
    );
}

#[test]
fn search_with_jq_invalid_filter_errors() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "test content").unwrap();

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--jq")
        .arg(".[invalid!!")
        .arg("test")
        .arg(tmp.path())
        .assert()
        .failure();
}

#[test]
fn eval_from_file() {
    let tmp = tempfile::tempdir().unwrap();
    let json_file = tmp.path().join("data.json");
    fs::write(&json_file, r#"{"name":"pore","version":"0.2.0"}"#).unwrap();

    let output = pore()
        .arg("eval")
        .arg(".name")
        .arg(&json_file)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("pore"),
        "Expected 'pore' in eval output: {}",
        stdout
    );
}

#[test]
fn eval_invalid_filter_errors() {
    pore().arg("eval").arg(".[bad syntax!!").assert().failure();
}

#[test]
fn aggregate_flag_groups_matches_by_extension() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.rs"), "hello rust").unwrap();
    fs::write(tmp.path().join("b.rs"), "hello again").unwrap();
    fs::write(tmp.path().join("c.txt"), "hello text").unwrap();

    let (mut cmd, _home) = pore_with_home();
    let output = cmd
        .arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--aggregate")
        .arg("ext")
        .arg("hello")
        .arg(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected JSON, got {stdout}: {e}"));
    let buckets = value["ext"]["buckets"].as_array().unwrap();
    let rs = buckets.iter().find(|b| b["key"] == "rs").unwrap();
    assert_eq!(rs["doc_count"], 2);
}

#[test]
fn aggregate_flag_on_unknown_field_fails_with_the_field_name() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.rs"), "hello").unwrap();

    let (mut cmd, _home) = pore_with_home();
    cmd.arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--aggregate")
        .arg("nope")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("nope"));
}

/// Builds a fixture with a .rs and a .txt file and runs one query against it.
fn query_files(query: &str) -> (std::process::Output, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("a.txt"), "the big bad wolf\n").unwrap();
    fs::write(tmp.path().join("src/main.rs"), "fn main() { wolf }\n").unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = pore()
        .env("HOME", home.path())
        .arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--color")
        .arg("never")
        .arg("-l")
        .arg(query)
        .arg(tmp.path())
        .output()
        .unwrap();
    (out, tmp)
}

#[test]
fn regex_query_on_contents_matches_a_term() {
    let (out, _t) = query_files("contents:/w.lf/");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("a.txt"),
        "regex should match the term 'wolf': {stdout}"
    );
}

#[test]
fn regex_query_on_filepath_selects_by_extension() {
    let (out, _t) = query_files(r"filepath:/.*\.rs/");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("main.rs"),
        "should match the .rs file: {stdout}"
    );
    assert!(
        !stdout.contains("a.txt"),
        "should not match the .txt file: {stdout}"
    );
}

#[test]
fn field_scoped_boolean_group_matches() {
    let (out, _t) = query_files("contents:(big AND bad)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("a.txt"),
        "field grouping should match: {stdout}"
    );
}

#[test]
fn filepath_regex_combines_with_a_content_term() {
    let (out, _t) = query_files(r"filepath:/.*\.rs/ AND wolf");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("main.rs"),
        "should combine field regex with a term: {stdout}"
    );
    assert!(
        !stdout.contains("a.txt"),
        "a.txt has the term but not the extension: {stdout}"
    );
}

#[test]
fn bare_regex_without_a_field_is_rejected_clearly() {
    // Pins a Tantivy limitation rather than a pore choice: a regex must name a field.
    // This is why the README cannot document `pore search "/b.* wolf/"`.
    let (out, _t) = query_files("/b.* wolf/");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "a bare regex must not silently succeed"
    );
    assert!(
        stderr.contains("specific field"),
        "error should say a regex needs a field, got: {stderr}"
    );
}

/// ROADMAP item 1: pore hard-required `HOME`, which Windows does not set, so every
/// command that touched config or cache failed with a bare
/// "environment variable not found". These run the CLI with `HOME` genuinely absent
/// rather than injecting one, which is what the rest of this file does.
///
/// `USERPROFILE` is set explicitly rather than left to the platform. Setting it is what
/// makes this the *Windows* scenario -- HOME missing, USERPROFILE present -- on every
/// platform. Relying on the ambient environment made these pass on Windows and fail on
/// Linux CI, where nothing defines USERPROFILE and so nothing resolves.
#[test]
fn works_with_home_unset() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();
    let profile = tempfile::tempdir().unwrap();

    pore()
        .env_remove("HOME")
        .env("USERPROFILE", profile.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn files_command_works_with_home_unset() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();
    let profile = tempfile::tempdir().unwrap();

    pore()
        .env_remove("HOME")
        .env("USERPROFILE", profile.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .arg("search")
        .arg("--in-memory")
        .arg("--files")
        .arg("")
        .arg(tmp.path())
        .assert()
        .success();
}

#[test]
fn unresolvable_home_names_the_variables_it_tried() {
    // With every candidate removed the lookup genuinely cannot succeed. The error must
    // say which variables were consulted -- the old one was just
    // "environment variable not found", which named nothing and suggested nothing.
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello world").unwrap();

    let assert = pore()
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("hello")
        .arg(tmp.path())
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("XDG_CONFIG_HOME") && stderr.contains("USERPROFILE"),
        "error should name the variables it tried, got: {stderr}"
    );
}

/// Runs a command that always fails, with `RUST_BACKTRACE` set as given.
fn failing_command_stderr(backtrace: Option<&str>) -> String {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.txt"), "hello").unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = pore();
    cmd.env("HOME", home.path());
    match backtrace {
        // The CI test job sets RUST_BACKTRACE=1, so the "off" case must remove it
        // rather than rely on it being absent.
        None => {
            cmd.env_remove("RUST_BACKTRACE");
            cmd.env_remove("RUST_LIB_BACKTRACE");
        }
        Some(v) => {
            cmd.env("RUST_BACKTRACE", v);
        }
    }
    let out = cmd
        .arg("search")
        .arg("--in-memory")
        .arg("--rebuild")
        .arg("--aggregate")
        .arg("bogus")
        .arg("hello")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(!out.status.success(), "this command is supposed to fail");
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[test]
fn errors_do_not_print_a_disabled_backtrace_placeholder() {
    // main printed err.backtrace() unconditionally, and Backtrace renders as the
    // literal "<disabled>" when capture is off -- which is the default.
    for bt in [None, Some("0")] {
        let stderr = failing_command_stderr(bt);
        assert!(
            stderr.contains("cannot aggregate on unknown field"),
            "the actual error must still be shown: {stderr}"
        );
        assert!(
            !stderr.contains("<disabled>"),
            "RUST_BACKTRACE={bt:?} must not print a placeholder: {stderr}"
        );
    }
}

#[test]
fn rust_backtrace_1_still_yields_a_usable_backtrace() {
    let stderr = failing_command_stderr(Some("1"));
    assert!(
        stderr.contains("cannot aggregate on unknown field"),
        "the error must still be shown: {stderr}"
    );
    assert!(
        stderr.contains("pore"),
        "a captured backtrace should name our own frames: {stderr}"
    );
    assert!(
        stderr.lines().count() > 3,
        "a backtrace should be readable multi-line output, not one blob: {stderr}"
    );
}
