//! Integration tests for `lidc init`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn init_creates_expected_files() {
    let dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["--root", dir.path().to_str().unwrap(), "init"])
        .assert()
        .success()
        .stdout(contains("Initialized LID project"));

    assert!(dir.path().join("docs/arrows/index.yaml").exists());
    assert!(dir.path().join("docs/arrows/core/overview.md").exists());
    assert!(dir.path().join("docs/intent").is_dir());

    let index = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();
    assert!(index.contains("schema_version: 2"));
    assert!(index.contains("core:"));
    assert!(index.contains("UNMAPPED"));

    let agents = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("Version: 1.3.0"));
    assert!(agents.contains("Memory vs. intent."));
    assert!(dir.path().join("CLAUDE.md").exists());
}

#[test]
fn init_custom_segment_name() {
    let dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args([
            "--root",
            dir.path().to_str().unwrap(),
            "init",
            "--segment",
            "billing",
        ])
        .assert()
        .success();

    let index = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();
    assert!(index.contains("billing:"));
    assert!(dir.path().join("docs/arrows/billing/overview.md").exists());
}

#[test]
fn init_fails_if_project_already_exists() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    fs::write(
        dir.path().join("docs/arrows/index.yaml"),
        "schema_version: 2\narrows: {}\n",
    )
    .unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["--root", dir.path().to_str().unwrap(), "init"])
        .assert()
        .code(2)
        .stderr(contains("already exists"));
}

#[test]
fn init_then_check_passes() {
    let dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("lidc")
        .unwrap()
        .args(["--root", dir.path().to_str().unwrap(), "init"])
        .assert()
        .success();

    // A freshly initialised project must pass coherence checks.
    Command::cargo_bin("lidc")
        .unwrap()
        .args(["--root", dir.path().to_str().unwrap(), "check"])
        .assert()
        .success();
}
