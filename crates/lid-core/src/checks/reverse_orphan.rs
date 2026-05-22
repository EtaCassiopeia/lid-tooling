//! Reverse-orphan check.
//!
//! Implements §4 (drift signals) of the upstream audit-checklist:
//! "reverse orphans — `@spec` annotations in code or tests that
//! reference spec IDs not present in any spec file."
//!
//! The methodology is explicit that reverse orphans are *asks*, not
//! fixes: the right resolution (create the spec, delete the
//! annotation, treat as alias) depends on user judgement. The CLI
//! surfaces them at Error severity so they're loud, but the
//! arrow-maintenance workflow asks the user before auto-resolving.

use std::collections::BTreeSet;

use crate::LidRepo;
use crate::model::SpecId;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct ReverseOrphanCheck;

impl Check for ReverseOrphanCheck {
    fn id(&self) -> CheckId {
        CheckId::ReverseOrphan
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let known: BTreeSet<&SpecId> = repo
            .specs
            .iter()
            .flat_map(|f| f.specs.iter().map(|s| &s.id))
            .collect();

        let mut findings = Vec::new();
        for c in &repo.citations {
            if known.contains(&c.id) {
                continue;
            }
            findings.push(Finding {
                check: CheckId::ReverseOrphan,
                severity: Severity::Error,
                category: Category::Orphans,
                message: format!(
                    "`@spec {}` references a spec ID that is not defined in any spec file",
                    c.id
                ),
                location: Some(Location {
                    path: c.file.clone(),
                    line: Some(c.line),
                }),
                spec: Some(c.id.clone()),
                remediation: Some(format!(
                    "either add `{}` to a spec file, delete the annotation, or alias it to an existing spec",
                    c.id
                )),
            });
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
        ArrowIndex, CitationKind, SpecCitation, SpecFile, SpecLine, SpecStatus, Unmapped,
    };

    fn citation(id: &str, file: &str, line: usize) -> SpecCitation {
        SpecCitation {
            id: SpecId::parse(id).unwrap(),
            file: PathBuf::from(file),
            line,
            kind: CitationKind::Code,
        }
    }

    fn spec_line(id: &str, line: usize) -> SpecLine {
        SpecLine {
            id: SpecId::parse(id).unwrap(),
            status: SpecStatus::Implemented,
            text: "stub".into(),
            line,
        }
    }

    fn make_repo(spec_ids: &[&str], citations: Vec<SpecCitation>) -> LidRepo {
        let root = PathBuf::from("/fake/root");
        let specs = if spec_ids.is_empty() {
            vec![]
        } else {
            vec![SpecFile {
                path: root.join("docs/specs/auth-specs.md"),
                specs: spec_ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| spec_line(id, i + 1))
                    .collect(),
                implementing_artifacts: vec![],
                lld: None,
            }]
        };
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
            citations,
        }
    }

    #[test]
    fn citation_to_known_spec_is_silent() {
        let repo = make_repo(&["AUTH-001"], vec![citation("AUTH-001", "src/auth.rs", 12)]);
        let findings = ReverseOrphanCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn citation_to_unknown_spec_is_flagged() {
        let repo = make_repo(&["AUTH-001"], vec![citation("AUTH-999", "src/auth.rs", 42)]);
        let findings = ReverseOrphanCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].category, Category::Orphans);
        assert!(findings[0].message.contains("AUTH-999"));
        let loc = findings[0].location.as_ref().unwrap();
        assert_eq!(loc.path, PathBuf::from("src/auth.rs"));
        assert_eq!(loc.line, Some(42));
        assert_eq!(findings[0].spec.as_ref().unwrap().as_str(), "AUTH-999");
    }

    #[test]
    fn each_orphan_citation_gets_its_own_finding() {
        let repo = make_repo(
            &[],
            vec![
                citation("A-001", "src/a.rs", 1),
                citation("B-001", "src/b.rs", 5),
                citation("C-001", "src/c.rs", 9),
            ],
        );
        let findings = ReverseOrphanCheck.run(&repo);
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn duplicate_citations_to_same_unknown_spec_each_flagged() {
        let repo = make_repo(
            &[],
            vec![
                citation("AUTH-999", "src/a.rs", 1),
                citation("AUTH-999", "src/b.rs", 2),
            ],
        );
        let findings = ReverseOrphanCheck.run(&repo);
        assert_eq!(findings.len(), 2, "each call site should surface");
    }

    #[test]
    fn check_id_is_reverse_orphan() {
        assert_eq!(ReverseOrphanCheck.id(), CheckId::ReverseOrphan);
    }
}
