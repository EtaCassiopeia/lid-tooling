//! LLD `Decisions & Alternatives` table check.
//!
//! Implements §11 of the audit-checklist — validation of the
//! structured slice every LLD is expected to carry:
//!
//! * An LLD with no `## Decisions & Alternatives` rows produces a
//!   Warning. Either the section is missing or the table is empty;
//!   in both cases the LLD doesn't carry the design rationale the
//!   methodology asks for.
//! * An LLD whose rows are *mostly* `[inferred]` (more than half)
//!   produces an Info finding. Inferred content is a brownfield
//!   convention indicating rows reconstructed from existing code
//!   rather than authored — a high concentration is fine as a
//!   transient state but should be triaged toward confirmation.
//!
//! Detecting stale `[inferred]` rows (unchanged for months) requires
//! git history and lands with the Wave 3 staleness checks.

use crate::LidRepo;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct LldDecisionsCheck;

impl Check for LldDecisionsCheck {
    fn id(&self) -> CheckId {
        CheckId::LldDecisions
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let mut findings = Vec::new();
        for lld in &repo.llds {
            let total = lld.decisions.len();
            let location = Some(Location {
                path: lld.path.clone(),
                line: None,
            });

            if total == 0 {
                findings.push(Finding {
                    check: CheckId::LldDecisions,
                    severity: Severity::Warning,
                    category: Category::Lint,
                    message: format!(
                        "LLD `{}` has no `## Decisions & Alternatives` rows",
                        display_relative(repo, &lld.path)
                    ),
                    location: location.clone(),
                    spec: None,
                    remediation: Some(
                        "add a `## Decisions & Alternatives` table with rows recording the key design choices"
                            .to_owned(),
                    ),
                });
                continue;
            }

            let inferred = lld.decisions.iter().filter(|d| d.inferred).count();
            if inferred * 2 > total {
                findings.push(Finding {
                    check: CheckId::LldDecisions,
                    severity: Severity::Info,
                    category: Category::Lint,
                    message: format!(
                        "LLD `{}` has {inferred}/{total} `[inferred]` rows — brownfield content awaiting human confirmation",
                        display_relative(repo, &lld.path)
                    ),
                    location,
                    spec: None,
                    remediation: Some(
                        "confirm or refute each `[inferred]` row by removing the marker once the design choice is validated"
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
    use std::path::PathBuf;

    use super::*;
    use crate::model::{ArrowIndex, DecisionRow, LldDoc, Unmapped};

    fn decision(inferred: bool) -> DecisionRow {
        DecisionRow {
            decision: "x".into(),
            chosen: "y".into(),
            alternatives: "z".into(),
            rationale: "ok".into(),
            inferred,
        }
    }

    fn make_repo(llds: Vec<LldDoc>) -> LidRepo {
        LidRepo {
            root: PathBuf::from("/fake/root"),
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs: vec![],
            llds,
            arrow_docs: vec![],
            citations: vec![],
        }
    }

    fn lld(rel: &str, rows: Vec<DecisionRow>) -> LldDoc {
        LldDoc {
            path: PathBuf::from("/fake/root").join(rel),
            decisions: rows,
        }
    }

    #[test]
    fn lld_with_authored_decisions_is_silent() {
        let repo = make_repo(vec![lld(
            "docs/llds/auth.md",
            vec![decision(false), decision(false), decision(false)],
        )]);
        let f = LldDecisionsCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn lld_with_empty_decisions_warns() {
        let repo = make_repo(vec![lld("docs/llds/empty.md", vec![])]);
        let f = LldDecisionsCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warning);
        assert!(f[0].message.contains("empty.md"));
        assert!(
            f[0].message
                .contains("no `## Decisions & Alternatives` rows")
        );
    }

    #[test]
    fn lld_majority_inferred_emits_info() {
        let repo = make_repo(vec![lld(
            "docs/llds/brownfield.md",
            vec![decision(true), decision(true), decision(false)],
        )]);
        let f = LldDecisionsCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
        assert!(f[0].message.contains("2/3"));
        assert!(f[0].message.contains("[inferred]"));
    }

    #[test]
    fn lld_with_half_inferred_does_not_warn() {
        // 1 of 2 is *not* "more than half" — boundary check.
        let repo = make_repo(vec![lld(
            "docs/llds/auth.md",
            vec![decision(true), decision(false)],
        )]);
        let f = LldDecisionsCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn lld_with_all_inferred_emits_info() {
        let repo = make_repo(vec![lld(
            "docs/llds/brownfield.md",
            vec![decision(true), decision(true)],
        )]);
        let f = LldDecisionsCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
    }

    #[test]
    fn each_lld_evaluated_independently() {
        let repo = make_repo(vec![
            lld("docs/llds/empty.md", vec![]),
            lld(
                "docs/llds/brownfield.md",
                vec![decision(true), decision(true), decision(false)],
            ),
            lld("docs/llds/clean.md", vec![decision(false)]),
        ]);
        let f = LldDecisionsCheck.run(&repo);
        // One Warning (empty) + one Info (brownfield majority).
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn check_id_is_lld_decisions() {
        assert_eq!(LldDecisionsCheck.id(), CheckId::LldDecisions);
    }
}
