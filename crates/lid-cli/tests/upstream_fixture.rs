//! End-to-end sanity test against a vendored snapshot of the upstream
//! LID repository's `docs/` tree.
//!
//! Purpose: prove the parsers, checks, and renderers don't crash or
//! produce nonsense findings on a real-world layout we didn't author.
//! The exact finding counts will drift as upstream evolves — the
//! fixture is a frozen snapshot — so assertions stay loose: structure
//! must be valid, exit codes must be sensible, and a few sanity
//! invariants must hold.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use assert_cmd::Command;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lid-upstream")
}

#[test]
fn lidc_check_runs_cleanly_against_upstream_fixture() {
    let output = Command::cargo_bin("lidc")
        .unwrap()
        .args(["check", "--json", "--root"])
        .arg(fixture_root())
        .output()
        .expect("failed to invoke lidc");

    // Exit must be 0 (no findings ≥ Error) or 1 (Error findings present),
    // never 2 (tool error / unable to parse).
    let code = output.status.code();
    assert!(
        matches!(code, Some(0 | 1)),
        "expected exit 0 or 1, got {code:?}; stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = std::str::from_utf8(&output.stdout).expect("stdout must be UTF-8");
    let report: serde_json::Value =
        serde_json::from_str(body).expect("--json output must be valid JSON");

    // The envelope is the contract; missing any of these keys means
    // a downstream consumer would break.
    assert!(report["tool"]["name"].is_string());
    assert!(report["tool"]["version"].is_string());
    assert!(report["summary"]["findings"].is_number());
    assert!(report["summary"]["by_severity"].is_object());
    assert!(report["summary"]["by_check"].is_object());
    assert!(report["findings"].is_array());

    let count = report["summary"]["findings"].as_u64().unwrap();
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(
        count,
        findings.len() as u64,
        "summary.findings must equal findings.len()"
    );
}

#[test]
fn upstream_fixture_has_no_schema_errors() {
    // Schema is the strictest check — if the upstream's index.yaml,
    // detail files, and taxonomy are well-formed, this stays empty.
    // The upstream is well-curated, so any schema finding here would
    // be either real drift in upstream or a parser regression.
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
        "schema check on upstream should be clean, got: {}",
        body_or_summary(&report)
    );
}

#[test]
fn upstream_fixture_has_no_reverse_orphan_errors() {
    // `@spec` mentions in HLDs / LLDs are illustrative prose, not
    // real references — they should not produce reverse-orphan
    // findings. This pins the documentation-extension behaviour of
    // `parse::source::classify_path`.
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
        "reverse-orphan check on upstream should be clean, got: {}",
        body_or_summary(&report)
    );
}

#[test]
fn upstream_fixture_has_no_reference_coherence_errors() {
    // Every `## References` bullet in upstream arrow docs should
    // resolve to a real file once decorated forms (backticks,
    // section anchors, em-dash trailers) are handled.
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
        "reference-coherence on upstream should be clean, got: {}",
        body_or_summary(&report)
    );
}

fn body_or_summary(report: &serde_json::Value) -> String {
    serde_json::to_string_pretty(&report["findings"]).unwrap_or_else(|_| "<unprintable>".into())
}
