//! Machine-readable JSON report.
//!
//! Stable top-level schema:
//!
//! ```json
//! {
//!   "tool":     { "name": "lidc", "version": "0.1.0" },
//!   "summary":  { "findings": 3, "by_severity": {"error": 2, "warning": 1}, "by_check": {…} },
//!   "findings": [ Finding, … ]
//! }
//! ```
//!
//! Each finding follows the shape defined in `lid_core::checks` and
//! omits `location` / `spec` / `remediation` when absent (the model
//! uses `skip_serializing_if = "Option::is_none"`), so consumers can
//! safely `jq` for shallow keys.

use std::collections::BTreeMap;

use lid_core::{Finding, Severity};
use serde::Serialize;

#[derive(Serialize)]
struct Report<'a> {
    tool: ToolInfo,
    summary: Summary,
    findings: &'a [Finding],
}

#[derive(Serialize)]
struct ToolInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct Summary {
    findings: usize,
    by_severity: BTreeMap<&'static str, usize>,
    by_check: BTreeMap<&'static str, usize>,
}

/// Render `findings` as pretty-printed JSON with a stable schema.
///
/// # Errors
/// Returns `serde_json::Error` only in the theoretical case that
/// serialisation fails; the value types involved here are all
/// owned-string / numeric so practical callers can `unwrap` safely.
pub fn render(findings: &[Finding]) -> Result<String, serde_json::Error> {
    let report = Report {
        tool: ToolInfo {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
        },
        summary: build_summary(findings),
        findings,
    };
    serde_json::to_string_pretty(&report)
}

fn build_summary(findings: &[Finding]) -> Summary {
    let mut by_severity: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut by_check: BTreeMap<&'static str, usize> = BTreeMap::new();
    for f in findings {
        *by_severity.entry(severity_key(f.severity)).or_default() += 1;
        *by_check.entry(f.check.as_str()).or_default() += 1;
    }
    Summary {
        findings: findings.len(),
        by_severity,
        by_check,
    }
}

fn severity_key(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use lid_core::{Category, CheckId, Finding, Location, Severity, SpecId};
    use serde_json::Value;

    use super::*;

    fn finding(severity: Severity, check: CheckId, message: &str) -> Finding {
        Finding {
            check,
            severity,
            category: Category::Schema,
            message: message.to_owned(),
            location: Some(Location {
                path: PathBuf::from("/repo/src/auth.rs"),
                line: Some(42),
            }),
            spec: None,
            remediation: None,
        }
    }

    fn json(findings: &[Finding]) -> Value {
        serde_json::from_str(&render(findings).unwrap()).unwrap()
    }

    #[test]
    fn empty_findings_render_with_zero_summary() {
        let v = json(&[]);
        assert_eq!(v["summary"]["findings"], 0);
        assert_eq!(v["findings"].as_array().unwrap().len(), 0);
        // Empty by_severity / by_check maps are still present (as objects)
        // so consumers don't have to special-case "no key" vs "zero count".
        assert!(v["summary"]["by_severity"].is_object());
        assert!(v["summary"]["by_check"].is_object());
    }

    #[test]
    fn tool_info_carries_crate_name_and_version() {
        let v = json(&[]);
        assert_eq!(v["tool"]["name"], "lid-cli");
        // Version comes from the crate manifest; just check it's a non-empty string.
        assert!(
            v["tool"]["version"]
                .as_str()
                .unwrap()
                .chars()
                .any(|c| c.is_ascii_digit())
        );
    }

    #[test]
    fn summary_counts_severities() {
        let findings = vec![
            finding(Severity::Error, CheckId::Schema, "a"),
            finding(Severity::Error, CheckId::Orphan, "b"),
            finding(Severity::Warning, CheckId::Orphan, "c"),
        ];
        let v = json(&findings);
        assert_eq!(v["summary"]["findings"], 3);
        assert_eq!(v["summary"]["by_severity"]["error"], 2);
        assert_eq!(v["summary"]["by_severity"]["warning"], 1);
        assert!(v["summary"]["by_severity"].get("info").is_none());
    }

    #[test]
    fn summary_counts_checks_by_kebab_id() {
        let findings = vec![
            finding(Severity::Error, CheckId::ReverseOrphan, "a"),
            finding(Severity::Error, CheckId::ReverseOrphan, "b"),
            finding(Severity::Warning, CheckId::Orphan, "c"),
        ];
        let v = json(&findings);
        assert_eq!(v["summary"]["by_check"]["reverse-orphan"], 2);
        assert_eq!(v["summary"]["by_check"]["orphan"], 1);
    }

    #[test]
    fn finding_includes_spec_field_when_present() {
        let mut f = finding(Severity::Error, CheckId::ReverseOrphan, "x");
        f.spec = Some(SpecId::parse("AUTH-001").unwrap());
        let v = json(&[f]);
        assert_eq!(v["findings"][0]["spec"], "AUTH-001");
    }

    #[test]
    fn finding_omits_optional_fields_when_absent() {
        let mut f = finding(Severity::Error, CheckId::Schema, "x");
        f.location = None;
        f.spec = None;
        f.remediation = None;
        let v = json(&[f]);
        let obj = v["findings"][0].as_object().unwrap();
        assert!(!obj.contains_key("location"));
        assert!(!obj.contains_key("spec"));
        assert!(!obj.contains_key("remediation"));
    }

    #[test]
    fn output_is_pretty_printed_with_newlines() {
        let s = render(&[finding(Severity::Error, CheckId::Schema, "x")]).unwrap();
        assert!(s.contains('\n'), "expected pretty-printed JSON");
    }
}
