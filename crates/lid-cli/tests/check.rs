//! Integration tests for `lidc check`.
//!
//! Each test creates a tempdir with a minimal LID-shaped layout and
//! invokes the binary via `assert_cmd`, asserting on exit codes and
//! stdout/stderr predicates. These tests exercise the full pipeline:
//! clap parsing, `LidRepo::discover`, every check in `default_checks`,
//! and the plaintext renderer.

#![allow(clippy::unwrap_used, clippy::expect_used)] // tests

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::str::contains;

/// Write a minimal LID layout under `root`. Uses `schema_version: 2` with the
/// legacy flat file layout (`docs/specs/` + `docs/llds/`) — still loaded by
/// the current discovery code. Issue #1 + #3 will migrate these to
/// `docs/intent/`. The optional `extra_yaml` is appended to `index.yaml`.
fn make_minimal_repo(root: &Path, extra_yaml: &str) {
    fs::create_dir_all(root.join("docs/arrows")).unwrap();
    fs::create_dir_all(root.join("docs/specs")).unwrap();
    fs::create_dir_all(root.join("docs/llds")).unwrap();

    let mut index = String::from(
        "\
schema_version: 2
arrows:
  auth:
    status: MAPPED
    detail: auth.md
    blocks: []
    blockedBy: []
",
    );
    index.push_str(extra_yaml);
    fs::write(root.join("docs/arrows/index.yaml"), index).unwrap();

    fs::write(
        root.join("docs/arrows/auth.md"),
        "# Arrow: auth\n\n## References\n\n### LLD\n- docs/llds/auth.md\n\n### EARS\n- docs/specs/auth-specs.md\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/llds/auth.md"),
        "\
# LLD: auth

Some prose.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| Session store | Redis | Memcached | Latency |
",
    )
    .unwrap();
    fs::write(
        root.join("docs/specs/auth-specs.md"),
        "# auth\n\n- [x] **AUTH-001**: ok.\n",
    )
    .unwrap();

    // The `[x]` spec needs a test citation to satisfy `CoverageCheck`.
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/auth.test.ts"),
        "// @spec AUTH-001\nit('logs in', () => {});\n",
    )
    .unwrap();
}

#[test]
fn check_succeeds_on_clean_repo() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path(), "");

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(contains("no findings"));
}

#[test]
fn check_reports_schema_error_with_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    // Reference a segment that doesn't exist in `arrows:`.
    make_minimal_repo(
        dir.path(),
        "  session:\n    status: MAPPED\n    detail: session.md\n    blocks: [phantom-segment]\n    blockedBy: []\n",
    );
    // session.md needs to exist to avoid an unrelated "detail not found".
    fs::write(
        dir.path().join("docs/arrows/session.md"),
        "# Arrow: session\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--root"])
        .arg(dir.path())
        .assert()
        .code(1)
        .stdout(contains("phantom-segment"));
}

#[test]
fn check_reports_reverse_orphan_with_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path(), "");
    // Add a source file citing a spec that doesn't exist.
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/auth.rs"),
        "// @spec AUTH-999\nfn login() {}\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--root"])
        .arg(dir.path())
        .assert()
        .code(1)
        .stdout(contains("AUTH-999"));
}

#[test]
fn check_json_output_parses_and_carries_summary() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path(), "");
    // Plant a reverse-orphan so the output has a non-trivial finding.
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/auth.rs"),
        "// @spec AUTH-999\nfn login() {}\n",
    )
    .unwrap();

    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--json", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "stdout={:?}", output.stdout);
    let body = std::str::from_utf8(&output.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(body).expect("stdout must be valid JSON");
    assert!(v["tool"]["name"].is_string());
    assert_eq!(v["summary"]["findings"], 1);
    assert_eq!(v["summary"]["by_check"]["reverse-orphan"], 1);
    assert_eq!(v["findings"][0]["spec"], "AUTH-999");
}

#[test]
fn check_exits_two_when_no_lid_repo_present() {
    let dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--root"])
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(contains("LID repo"));
}

#[test]
fn only_filter_runs_just_the_listed_check() {
    let dir = tempfile::tempdir().unwrap();
    // This layout has a reverse orphan (caught by reverse-orphan) but no
    // schema or reference issues. `--only schema` should report nothing.
    make_minimal_repo(dir.path(), "");
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/auth.rs"),
        "// @spec AUTH-999\nfn login() {}\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--only", "schema", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(contains("no findings"));
}

#[test]
fn fail_on_warning_promotes_orphan_to_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path(), "");
    // Add an LLD that no arrow doc references — caught by `orphan`,
    // which is Warning severity.
    fs::write(dir.path().join("docs/llds/lonely.md"), "# LLD: lonely\n").unwrap();

    // Default `--fail-on error` => exit 0 (only a Warning present).
    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--root"])
        .arg(dir.path())
        .assert()
        .success();

    // `--fail-on warning` => exit 1.
    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--fail-on", "warning", "--root"])
        .arg(dir.path())
        .assert()
        .code(1);
}

#[test]
fn check_exits_two_on_unsupported_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    fs::write(
        dir.path().join("docs/arrows/index.yaml"),
        "schema_version: 1\narrows: {}\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--root"])
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(contains("schema_version 1 is not supported"))
        .stderr(contains("supported: [2]"));
}

#[test]
fn invalid_only_value_exits_two_with_helpful_error() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path(), "");

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--only", "bogus-check", "--root"])
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(contains("bogus-check"));
}
