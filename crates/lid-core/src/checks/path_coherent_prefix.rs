//! Path-prefix coherence check.
//!
//! A spec at `docs/intent/{a}/{b}/…-specs.md` should declare `prefix: A-B`
//! (folder names uppercased and joined by hyphens). This check warns when the
//! declared prefix doesn't match what the path implies, so teams can adopt the
//! LID 1.2.0 convention incrementally without an immediate hard failure.

use std::path::Path;

use crate::LidRepo;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct PathCoherentPrefixCheck;

impl Check for PathCoherentPrefixCheck {
    fn id(&self) -> CheckId {
        CheckId::PathCoherentPrefix
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let mut findings = Vec::new();

        for spec_file in &repo.specs {
            let Some(declared) = &spec_file.prefix else {
                continue;
            };
            let Some(expected) = derive_prefix(&repo.root, &spec_file.path) else {
                continue;
            };
            if declared != &expected {
                findings.push(Finding {
                    check: CheckId::PathCoherentPrefix,
                    severity: Severity::Warning,
                    category: Category::Schema,
                    message: format!(
                        "declared prefix `{declared}` does not match path-derived prefix `{expected}`"
                    ),
                    location: Some(Location {
                        path: spec_file.path.clone(),
                        line: None,
                    }),
                    spec: None,
                    remediation: Some(format!(
                        "update `prefix:` frontmatter to `{expected}` or move the file to the matching `docs/intent/` folder"
                    )),
                });
            }
        }

        findings
    }
}

/// Derive the expected spec prefix from an absolute spec file path.
///
/// Strips `<root>/docs/intent/` and uppercases all remaining directory
/// components (not the filename itself), joining them with `-`. Returns
/// `None` when the file is not under `docs/intent/` or sits directly in
/// the intent root with no segment subdirectory.
pub(crate) fn derive_prefix(root: &Path, spec_path: &Path) -> Option<String> {
    let rel = spec_path.strip_prefix(root).ok()?;
    let under_intent = rel.strip_prefix("docs/intent").ok()?;
    let dir = under_intent.parent()?;
    let parts: Vec<String> = dir
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .filter(|s| !s.is_empty())
        .map(str::to_uppercase)
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("-"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::LidRepo;
    use crate::model::{ArrowIndex, SpecFile, SpecLine, SpecStatus, Unmapped};

    fn spec_line(id: &str) -> SpecLine {
        SpecLine {
            id: crate::model::SpecId::parse(id).unwrap(),
            status: SpecStatus::Open,
            text: "stub".into(),
            line: 1,
        }
    }

    fn make_repo(root: &str, files: Vec<(&str, Option<&str>)>) -> LidRepo {
        let root = PathBuf::from(root);
        let specs = files
            .into_iter()
            .map(|(rel_path, prefix)| SpecFile {
                path: root.join(rel_path),
                specs: vec![],
                implementing_artifacts: vec![],
                lld: None,
                prefix: prefix.map(str::to_owned),
            })
            .collect();
        LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs,
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
            decision_docs: vec![],
        }
    }

    // ── derive_prefix unit tests ──────────────────────────────────────────────

    #[test]
    fn derives_single_folder_prefix() {
        let root = PathBuf::from("/repo");
        let path = root.join("docs/intent/auth/auth-specs.md");
        assert_eq!(derive_prefix(&root, &path).unwrap(), "AUTH");
    }

    #[test]
    fn derives_two_folder_prefix() {
        let root = PathBuf::from("/repo");
        let path = root.join("docs/intent/peval/run/run-specs.md");
        assert_eq!(derive_prefix(&root, &path).unwrap(), "PEVAL-RUN");
    }

    #[test]
    fn derives_three_folder_prefix() {
        let root = PathBuf::from("/repo");
        let path = root.join("docs/intent/checkout/payment/gateway/gateway-specs.md");
        assert_eq!(
            derive_prefix(&root, &path).unwrap(),
            "CHECKOUT-PAYMENT-GATEWAY"
        );
    }

    #[test]
    fn derives_prefix_with_hyphenated_folder_name() {
        let root = PathBuf::from("/repo");
        let path = root.join("docs/intent/shortener-core/alias-gen-specs.md");
        assert_eq!(derive_prefix(&root, &path).unwrap(), "SHORTENER-CORE");
    }

    #[test]
    fn returns_none_for_file_directly_in_intent_root() {
        let root = PathBuf::from("/repo");
        let path = root.join("docs/intent/auth-specs.md");
        assert!(derive_prefix(&root, &path).is_none());
    }

    #[test]
    fn returns_none_for_file_outside_intent_dir() {
        let root = PathBuf::from("/repo");
        let path = root.join("docs/specs/auth-specs.md");
        assert!(derive_prefix(&root, &path).is_none());
    }

    // ── check integration tests ───────────────────────────────────────────────

    #[test]
    fn check_id_is_path_coherent_prefix() {
        assert_eq!(PathCoherentPrefixCheck.id(), CheckId::PathCoherentPrefix);
    }

    #[test]
    fn no_finding_when_prefix_matches_single_folder() {
        let repo = make_repo(
            "/fake/root",
            vec![("docs/intent/auth/auth-specs.md", Some("AUTH"))],
        );
        assert!(PathCoherentPrefixCheck.run(&repo).is_empty());
    }

    #[test]
    fn no_finding_when_prefix_matches_nested_folder() {
        let repo = make_repo(
            "/fake/root",
            vec![("docs/intent/peval/run/run-specs.md", Some("PEVAL-RUN"))],
        );
        assert!(PathCoherentPrefixCheck.run(&repo).is_empty());
    }

    #[test]
    fn warning_when_declared_prefix_does_not_match_path() {
        let repo = make_repo(
            "/fake/root",
            vec![("docs/intent/auth/auth-specs.md", Some("STORE"))],
        );
        let findings = PathCoherentPrefixCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.check, CheckId::PathCoherentPrefix);
        assert!(f.message.contains("STORE"), "message: {}", f.message);
        assert!(f.message.contains("AUTH"), "message: {}", f.message);
    }

    #[test]
    fn warning_when_nested_prefix_partially_wrong() {
        let repo = make_repo(
            "/fake/root",
            vec![("docs/intent/peval/run/run-specs.md", Some("PEVAL"))],
        );
        let findings = PathCoherentPrefixCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("PEVAL-RUN"));
    }

    #[test]
    fn skips_spec_file_with_no_declared_prefix() {
        let repo = make_repo("/fake/root", vec![("docs/intent/auth/auth-specs.md", None)]);
        assert!(PathCoherentPrefixCheck.run(&repo).is_empty());
    }

    #[test]
    fn skips_spec_file_outside_intent_dir() {
        let repo = make_repo(
            "/fake/root",
            vec![("docs/specs/auth-specs.md", Some("AUTH"))],
        );
        assert!(PathCoherentPrefixCheck.run(&repo).is_empty());
    }

    #[test]
    fn each_mismatched_file_gets_its_own_finding() {
        let repo = make_repo(
            "/fake/root",
            vec![
                ("docs/intent/auth/auth-specs.md", Some("WRONG")),
                ("docs/intent/api/api-specs.md", Some("WRONG")),
                ("docs/intent/billing/billing-specs.md", Some("BILLING")),
            ],
        );
        let findings = PathCoherentPrefixCheck.run(&repo);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn finding_location_points_at_spec_file() {
        let repo = make_repo(
            "/fake/root",
            vec![("docs/intent/auth/auth-specs.md", Some("WRONG"))],
        );
        let findings = PathCoherentPrefixCheck.run(&repo);
        let loc = findings[0].location.as_ref().unwrap();
        assert!(loc.path.ends_with("auth-specs.md"));
        assert!(loc.line.is_none());
    }

    #[test]
    fn finding_includes_remediation_hint() {
        let repo = make_repo(
            "/fake/root",
            vec![("docs/intent/auth/auth-specs.md", Some("WRONG"))],
        );
        let findings = PathCoherentPrefixCheck.run(&repo);
        assert!(findings[0].remediation.is_some());
    }

    #[test]
    fn spec_file_produces_spec_line_using_helper() {
        // Ensure the spec_line helper itself doesn't panic.
        let line = spec_line("AUTH-001");
        assert_eq!(line.id.as_str(), "AUTH-001");
    }
}
