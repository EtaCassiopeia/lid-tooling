//! Integration tests for `lidc check`.
//!
//! Each test creates a tempdir with a minimal LID-shaped layout and
//! invokes the binary via `assert_cmd`, asserting on exit codes and
//! stdout/stderr predicates. These tests exercise the full pipeline:
//! clap parsing, `LidRepo::discover`, every check in `default_checks`,
//! and the plaintext renderer.

#![allow(clippy::unwrap_used)] // tests

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::str::contains;

/// Write a minimal LID layout under `root`. The optional `extra_yaml`
/// is appended to the bottom of `index.yaml` so tests can introduce
/// schema-level breakage without rewriting the whole file.
fn make_minimal_repo(root: &Path, extra_yaml: &str) {
    fs::create_dir_all(root.join("docs/arrows")).unwrap();
    fs::create_dir_all(root.join("docs/specs")).unwrap();
    fs::create_dir_all(root.join("docs/llds")).unwrap();

    let mut index = String::from(
        "\
schema_version: 1
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
        "# LLD: auth\n\nSome prose.\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/specs/auth-specs.md"),
        "# auth\n\n- [x] **AUTH-001**: ok.\n",
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
