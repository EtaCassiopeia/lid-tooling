//! Spec-ID format compliance / uniqueness check.
//!
//! Format validity is already enforced upstream — every `SpecId` in
//! `repo.specs` passed `SpecId::parse` at load time. What remains for
//! this check is the *uniqueness* clause from the methodology: spec
//! IDs are stable and globally unique across the project. Two spec
//! lines sharing an ID is a hard error.
//!
//! Reuse-after-deletion detection (the same ID resurrected after a
//! previous deletion) needs git history and lands with the Wave 3
//! staleness checks.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::LidRepo;
use crate::model::SpecId;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct SpecIdFormatCheck;

impl Check for SpecIdFormatCheck {
    fn id(&self) -> CheckId {
        CheckId::SpecIdFormat
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        // Spec ID -> ordered list of every occurrence (file, line).
        let mut occurrences: BTreeMap<&SpecId, Vec<(&PathBuf, usize)>> = BTreeMap::new();
        for spec_file in &repo.specs {
            for line in &spec_file.specs {
                occurrences
                    .entry(&line.id)
                    .or_default()
                    .push((&spec_file.path, line.line));
            }
        }

        let mut findings = Vec::new();
        for (id, locs) in &occurrences {
            if locs.len() < 2 {
                continue;
            }
            // First occurrence is treated as the "primary" definition;
            // each subsequent occurrence is the duplicate finding.
            let (primary_path, primary_line) = locs[0];
            for (dup_path, dup_line) in locs.iter().skip(1) {
                findings.push(Finding {
                    check: CheckId::SpecIdFormat,
                    severity: Severity::Error,
                    category: Category::Schema,
                    message: format!(
                        "duplicate spec ID `{id}` (first defined at `{}`:{primary_line})",
                        display_relative(repo, primary_path),
                    ),
                    location: Some(Location {
                        path: (*dup_path).clone(),
                        line: Some(*dup_line),
                    }),
                    spec: Some((*id).clone()),
                    remediation: Some(
                        "rename one of the occurrences; spec IDs are unique across the project"
                            .to_owned(),
                    ),
                });
            }
        }
        findings
    }
}

fn display_relative(repo: &LidRepo, path: &std::path::Path) -> String {
    path.strip_prefix(&repo.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::model::{ArrowIndex, SpecFile, SpecLine, SpecStatus, Unmapped};

    fn spec_line(id: &str, line: usize) -> SpecLine {
        SpecLine {
            id: SpecId::parse(id).unwrap(),
            status: SpecStatus::Implemented,
            text: "stub".into(),
            line,
        }
    }

    fn make_repo(files: Vec<(&str, Vec<SpecLine>)>) -> LidRepo {
        let root = PathBuf::from("/fake/root");
        let specs = files
            .into_iter()
            .map(|(name, lines)| SpecFile {
                path: root.join("docs/specs").join(name),
                specs: lines,
                implementing_artifacts: vec![],
                lld: None,
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
        }
    }

    #[test]
    fn unique_ids_yield_no_findings() {
        let repo = make_repo(vec![
            (
                "auth-specs.md",
                vec![spec_line("AUTH-001", 5), spec_line("AUTH-002", 6)],
            ),
            ("billing-specs.md", vec![spec_line("BILL-001", 5)]),
        ]);
        let findings = SpecIdFormatCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn duplicate_within_same_file_is_flagged() {
        let repo = make_repo(vec![(
            "auth-specs.md",
            vec![spec_line("AUTH-001", 5), spec_line("AUTH-001", 12)],
        )]);
        let findings = SpecIdFormatCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.category, Category::Schema);
        assert!(f.message.contains("AUTH-001"));
        let loc = f.location.as_ref().unwrap();
        // The *second* occurrence is reported as the duplicate.
        assert_eq!(loc.line, Some(12));
    }

    #[test]
    fn duplicate_across_files_is_flagged() {
        let repo = make_repo(vec![
            ("auth-specs.md", vec![spec_line("AUTH-001", 7)]),
            ("billing-specs.md", vec![spec_line("AUTH-001", 3)]),
        ]);
        let findings = SpecIdFormatCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        // Primary is auth-specs (first by sorted path); duplicate is in billing.
        let loc = findings[0].location.as_ref().unwrap();
        assert!(loc.path.ends_with("billing-specs.md"), "loc={loc:?}");
        assert!(
            findings[0].message.contains("auth-specs.md"),
            "expected message to mention the primary location, got {}",
            findings[0].message
        );
    }

    #[test]
    fn triple_occurrence_produces_two_findings() {
        let repo = make_repo(vec![(
            "auth-specs.md",
            vec![
                spec_line("AUTH-001", 5),
                spec_line("AUTH-001", 7),
                spec_line("AUTH-001", 9),
            ],
        )]);
        let findings = SpecIdFormatCheck.run(&repo);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn finding_carries_the_offending_spec_id() {
        let repo = make_repo(vec![(
            "auth-specs.md",
            vec![spec_line("AUTH-001", 5), spec_line("AUTH-001", 12)],
        )]);
        let findings = SpecIdFormatCheck.run(&repo);
        assert_eq!(findings[0].spec.as_ref().unwrap().as_str(), "AUTH-001");
    }

    #[test]
    fn check_id_is_spec_id_format() {
        assert_eq!(SpecIdFormatCheck.id(), CheckId::SpecIdFormat);
    }
}
