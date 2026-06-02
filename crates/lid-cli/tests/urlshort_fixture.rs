//! End-to-end tests against the vendored `urlshort` example project fixture.
//!
//! The `urlshort` fixture is a minimal URL shortener project migrated to the
//! `schema_version: 2` / `docs/intent/` node-as-folder layout. It exercises the
//! flat (non-hierarchical) segment structure — no `children`/`parent` fields —
//! which is the baseline layout for new LID projects.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use assert_cmd::Command;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/urlshort")
}

#[test]
fn lidc_check_runs_cleanly_against_urlshort_fixture() {
    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--json", "--root"])
        .arg(fixture_root())
        .output()
        .expect("failed to invoke lidc");

    let code = output.status.code();
    assert!(
        matches!(code, Some(0 | 1)),
        "expected exit 0 or 1, got {code:?}; stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = std::str::from_utf8(&output.stdout).expect("stdout must be UTF-8");
    let report: serde_json::Value =
        serde_json::from_str(body).expect("--json output must be valid JSON");

    assert!(report["tool"]["name"].is_string());
    assert!(report["summary"]["findings"].is_number());
    assert!(report["findings"].is_array());
}

#[test]
fn urlshort_fixture_has_no_schema_errors() {
    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--json", "--only", "schema", "--root"])
        .arg(fixture_root())
        .output()
        .unwrap();

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["summary"]["findings"].as_u64().unwrap(),
        0,
        "schema check on urlshort should be clean, got: {}",
        serde_json::to_string_pretty(&report["findings"]).unwrap_or_default()
    );
}

#[test]
fn urlshort_fixture_has_no_reference_coherence_errors() {
    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--json", "--only", "reference-coherence", "--root"])
        .arg(fixture_root())
        .output()
        .unwrap();

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["summary"]["findings"].as_u64().unwrap(),
        0,
        "reference-coherence on urlshort should be clean, got: {}",
        serde_json::to_string_pretty(&report["findings"]).unwrap_or_default()
    );
}

#[test]
fn urlshort_fixture_has_no_path_coherent_prefix_errors() {
    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args([
            "check",
            "--json",
            "--only",
            "path-coherent-prefix",
            "--root",
        ])
        .arg(fixture_root())
        .output()
        .unwrap();

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["summary"]["findings"].as_u64().unwrap(),
        0,
        "path-coherent-prefix on urlshort should be clean, got: {}",
        serde_json::to_string_pretty(&report["findings"]).unwrap_or_default()
    );
}

#[test]
fn urlshort_fixture_has_no_reverse_orphan_errors() {
    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--json", "--only", "reverse-orphan", "--root"])
        .arg(fixture_root())
        .output()
        .unwrap();

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["summary"]["findings"].as_u64().unwrap(),
        0,
        "reverse-orphan on urlshort should be clean, got: {}",
        serde_json::to_string_pretty(&report["findings"]).unwrap_or_default()
    );
}
