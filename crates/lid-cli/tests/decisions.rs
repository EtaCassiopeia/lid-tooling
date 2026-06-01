//! Integration tests for `lidc decisions`.

#![allow(clippy::unwrap_used, clippy::expect_used)] // tests

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn make_minimal_repo(root: &Path) {
    fs::create_dir_all(root.join("docs/arrows")).unwrap();
    fs::create_dir_all(root.join("docs/intent/auth")).unwrap();
    fs::write(
        root.join("docs/arrows/index.yaml"),
        "schema_version: 2\narrows:\n  auth:\n    status: MAPPED\n    detail: auth.md\n",
    )
    .unwrap();
    fs::write(root.join("docs/arrows/auth.md"), "# Arrow: auth\n").unwrap();
    fs::write(root.join("docs/intent/auth/auth-specs.md"), "# auth\n").unwrap();
}

#[test]
fn decisions_empty_repo_prints_no_docs_found() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path());

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["--root", dir.path().to_str().unwrap(), "decisions"])
        .assert()
        .success()
        .stdout(contains("0 total"))
        .stdout(contains("No decision documents found."));
}

#[test]
fn decisions_lists_project_level_doc() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path());
    fs::create_dir_all(dir.path().join("docs/decisions")).unwrap();
    fs::write(
        dir.path().join("docs/decisions/arch.md"),
        "# Architecture Choice\n\nSome rationale.\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["--root", dir.path().to_str().unwrap(), "decisions"])
        .assert()
        .success()
        .stdout(contains("1 total"))
        .stdout(contains("Project"))
        .stdout(contains("arch.md"))
        .stdout(contains("Architecture Choice"));
}

#[test]
fn decisions_lists_per_node_doc() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path());
    fs::create_dir_all(dir.path().join("docs/intent/auth/decisions")).unwrap();
    fs::write(
        dir.path().join("docs/intent/auth/decisions/token.md"),
        "# Token Storage\n\nJWT vs opaque.\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["--root", dir.path().to_str().unwrap(), "decisions"])
        .assert()
        .success()
        .stdout(contains("1 total"))
        .stdout(contains("auth"))
        .stdout(contains("token.md"))
        .stdout(contains("Token Storage"));
}

#[test]
fn decisions_scope_filter_project_hides_node_docs() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path());
    fs::create_dir_all(dir.path().join("docs/decisions")).unwrap();
    fs::write(dir.path().join("docs/decisions/arch.md"), "# Arch\n").unwrap();
    fs::create_dir_all(dir.path().join("docs/intent/auth/decisions")).unwrap();
    fs::write(
        dir.path().join("docs/intent/auth/decisions/token.md"),
        "# Token\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args([
            "--root",
            dir.path().to_str().unwrap(),
            "decisions",
            "--scope",
            "project",
        ])
        .assert()
        .success()
        .stdout(contains("1 total"))
        .stdout(contains("arch.md"))
        .stdout(predicates::str::contains("token.md").not());
}

#[test]
fn decisions_scope_filter_node_hides_project_docs() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path());
    fs::create_dir_all(dir.path().join("docs/decisions")).unwrap();
    fs::write(dir.path().join("docs/decisions/arch.md"), "# Arch\n").unwrap();
    fs::create_dir_all(dir.path().join("docs/intent/auth/decisions")).unwrap();
    fs::write(
        dir.path().join("docs/intent/auth/decisions/token.md"),
        "# Token\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args([
            "--root",
            dir.path().to_str().unwrap(),
            "decisions",
            "--scope",
            "node",
        ])
        .assert()
        .success()
        .stdout(contains("1 total"))
        .stdout(contains("token.md"))
        .stdout(predicates::str::contains("arch.md").not());
}

#[test]
fn decisions_invalid_scope_exits_two() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path());

    Command::cargo_bin("lidc")
        .unwrap()
        .args([
            "--root",
            dir.path().to_str().unwrap(),
            "decisions",
            "--scope",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(contains("invalid --scope value"));
}

#[test]
fn decisions_json_output_is_valid_array() {
    let dir = tempfile::tempdir().unwrap();
    make_minimal_repo(dir.path());
    fs::create_dir_all(dir.path().join("docs/decisions")).unwrap();
    fs::write(
        dir.path().join("docs/decisions/arch.md"),
        "# Architecture\n",
    )
    .unwrap();

    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args([
            "--root",
            dir.path().to_str().unwrap(),
            "--json",
            "decisions",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("output must be valid JSON");
    assert!(parsed.is_array());
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["scope"], "project");
    assert_eq!(arr[0]["title"], "Architecture");
}
