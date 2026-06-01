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
//!
//! LID 1.2.0 exemption: an LLD whose parent directory contains a non-empty
//! `decisions/` subdirectory is exempt from the missing-table warning. The
//! design intent has been captured; it just lives in a standalone decision
//! doc rather than inline in the design doc.

use crate::LidRepo;
use crate::model::DecisionScope;

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
                // Exempt if the node folder has a per-node decisions/ directory
                // with at least one decision doc — the intent is captured there.
                let node_name = lld
                    .path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let has_decision_docs = repo.decision_docs.iter().any(
                    |d| matches!(&d.scope, DecisionScope::Node { segment } if segment == node_name),
                );
                if has_decision_docs {
                    continue;
                }

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
        make_repo_with_decisions(llds, vec![])
    }

    fn make_repo_with_decisions(
        llds: Vec<LldDoc>,
        decision_docs: Vec<crate::model::DecisionDoc>,
    ) -> LidRepo {
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
            decision_docs,
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

    #[test]
    fn lld_without_decisions_but_with_node_decisions_dir_is_silent() {
        use crate::model::{DecisionDoc, DecisionScope};
        // LLD at docs/intent/auth/auth-design.md with no table rows,
        // but there is a decision doc scoped to the same node.
        let decision_doc = DecisionDoc {
            path: PathBuf::from("/fake/root/docs/intent/auth/decisions/token.md"),
            scope: DecisionScope::Node {
                segment: "auth".to_owned(),
            },
            title: "Token Format".to_owned(),
        };
        let repo = make_repo_with_decisions(
            vec![lld("docs/intent/auth/auth-design.md", vec![])],
            vec![decision_doc],
        );
        let f = LldDecisionsCheck.run(&repo);
        assert!(
            f.is_empty(),
            "expected silence when decisions/ dir has docs, got {f:?}"
        );
    }

    #[test]
    fn lld_without_decisions_and_without_decisions_dir_still_warns() {
        // Same path shape but no decision docs.
        let repo = make_repo(vec![lld("docs/intent/auth/auth-design.md", vec![])]);
        let f = LldDecisionsCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warning);
    }
}
