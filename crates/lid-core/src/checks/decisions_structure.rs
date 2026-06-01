//! Coherence checks for standalone decision documents.
//!
//! Decision docs (`docs/decisions/` and `docs/intent/<node>/decisions/`)
//! must satisfy three structural rules:
//!
//! 1. Every decision doc must have an H1 title — a missing title is a Warning.
//! 2. Per-node decision docs must belong to a segment that is registered in
//!    `docs/arrows/index.yaml` — an orphaned node directory is a Warning.
//! 3. Decision docs must not contain `@spec` citations — "no EARS attached"
//!    is part of the LID 1.2.0 decision-doc contract.

use std::collections::BTreeSet;
use std::path::Path;

use crate::LidRepo;
use crate::model::DecisionScope;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct DecisionsStructureCheck;

impl Check for DecisionsStructureCheck {
    fn id(&self) -> CheckId {
        CheckId::DecisionsStructure
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Build a set of known segment names (lowercase) for the orphan check.
        let known_segments: BTreeSet<String> =
            repo.index.arrows.keys().map(ToString::to_string).collect();

        // Build a set of decision doc paths for the @spec citation check.
        let decision_paths: BTreeSet<&Path> = repo
            .decision_docs
            .iter()
            .map(|d| d.path.as_path())
            .collect();

        for doc in &repo.decision_docs {
            let rel = doc.path.strip_prefix(&repo.root).unwrap_or(&doc.path);
            let location = Some(Location {
                path: doc.path.clone(),
                line: None,
            });

            // Rule 1 — must have an H1 title.
            if doc.title.is_empty() {
                findings.push(Finding {
                    check: CheckId::DecisionsStructure,
                    severity: Severity::Warning,
                    category: Category::Lint,
                    message: format!(
                        "decision doc `{}` has no H1 title (`# Title`)",
                        rel.display()
                    ),
                    location: location.clone(),
                    spec: None,
                    remediation: Some(
                        "add a `# Title` heading as the first line of the decision doc".to_owned(),
                    ),
                });
            }

            // Rule 2 — per-node docs must belong to a known segment.
            if let DecisionScope::Node { segment } = &doc.scope {
                if !known_segments.contains(segment.as_str()) {
                    findings.push(Finding {
                        check: CheckId::DecisionsStructure,
                        severity: Severity::Warning,
                        category: Category::Orphans,
                        message: format!(
                            "decision doc `{}` is scoped to node `{segment}` which is not registered in docs/arrows/index.yaml",
                            rel.display()
                        ),
                        location: location.clone(),
                        spec: None,
                        remediation: Some(format!(
                            "add segment `{segment}` to docs/arrows/index.yaml or move the decision doc to docs/decisions/"
                        )),
                    });
                }
            }
        }

        // Rule 3 — no @spec citations in decision docs.
        for citation in &repo.citations {
            if decision_paths.contains(citation.file.as_path()) {
                let rel_cite = citation
                    .file
                    .strip_prefix(&repo.root)
                    .unwrap_or(&citation.file);
                findings.push(Finding {
                    check: CheckId::DecisionsStructure,
                    severity: Severity::Warning,
                    category: Category::Lint,
                    message: format!(
                        "decision doc `{}` contains `@spec {}` — decision docs carry no EARS specs",
                        rel_cite.display(),
                        citation.id
                    ),
                    location: Some(Location {
                        path: citation.file.clone(),
                        line: Some(citation.line),
                    }),
                    spec: Some(citation.id.clone()),
                    remediation: Some(
                        "remove the `@spec` annotation; decisions record choices, not implementations"
                            .to_owned(),
                    ),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::model::{
        ArrowIndex, CitationKind, DecisionDoc, DecisionScope, SpecCitation, SpecId, Unmapped,
    };

    fn make_repo(decision_docs: Vec<DecisionDoc>, citations: Vec<SpecCitation>) -> LidRepo {
        LidRepo {
            root: PathBuf::from("/fake/root"),
            index: ArrowIndex {
                schema_version: 2,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs: vec![],
            llds: vec![],
            arrow_docs: vec![],
            citations,
            decision_docs,
        }
    }

    fn project_doc(name: &str, title: &str) -> DecisionDoc {
        DecisionDoc {
            path: PathBuf::from("/fake/root/docs/decisions").join(format!("{name}.md")),
            scope: DecisionScope::Project,
            title: title.to_owned(),
        }
    }

    fn node_doc(segment: &str, name: &str, title: &str) -> DecisionDoc {
        DecisionDoc {
            path: PathBuf::from("/fake/root/docs/intent")
                .join(segment)
                .join("decisions")
                .join(format!("{name}.md")),
            scope: DecisionScope::Node {
                segment: segment.to_owned(),
            },
            title: title.to_owned(),
        }
    }

    #[test]
    fn valid_decision_doc_is_silent() {
        let repo = make_repo(vec![project_doc("arch", "Architecture Decision")], vec![]);
        let findings = DecisionsStructureCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn missing_title_emits_warning() {
        let repo = make_repo(vec![project_doc("arch", "")], vec![]);
        let findings = DecisionsStructureCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("no H1 title"));
    }

    #[test]
    fn per_node_doc_in_unknown_segment_emits_warning() {
        // The repo has no registered segments, so `auth` is unknown.
        let repo = make_repo(vec![node_doc("auth", "token", "Token Format")], vec![]);
        let findings = DecisionsStructureCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("auth"));
        assert_eq!(findings[0].category, Category::Orphans);
    }

    #[test]
    fn at_spec_citation_in_decision_doc_emits_warning() {
        let doc = project_doc("arch", "Architecture Decision");
        let citation = SpecCitation {
            id: SpecId::parse("AUTH-001").unwrap(),
            file: doc.path.clone(),
            line: 5,
            kind: CitationKind::Other,
        };
        let repo = make_repo(vec![doc], vec![citation]);
        let findings = DecisionsStructureCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("@spec AUTH-001"));
        assert!(findings[0].message.contains("carry no EARS specs"));
    }

    #[test]
    fn citation_in_non_decision_doc_is_not_flagged() {
        let doc = project_doc("arch", "Architecture Decision");
        // Citation is in a source file, not a decision doc.
        let citation = SpecCitation {
            id: SpecId::parse("AUTH-001").unwrap(),
            file: PathBuf::from("/fake/root/src/auth.rs"),
            line: 1,
            kind: CitationKind::Code,
        };
        let repo = make_repo(vec![doc], vec![citation]);
        let findings = DecisionsStructureCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn check_id_is_decisions_structure() {
        assert_eq!(DecisionsStructureCheck.id(), CheckId::DecisionsStructure);
    }
}
