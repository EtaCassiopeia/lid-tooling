//! Coverage check.
//!
//! Implements §2 of the upstream audit-checklist: every behavioural
//! EARS spec must have at least one eval assertion citing it. We
//! approximate "behavioural" with `SpecStatus::Implemented` — a spec
//! marked `[x]` claims to be implemented, so it should have a
//! corresponding test that cites the spec ID with `@spec`.
//!
//! `Open` (`[ ]`) and `Deferred` (`[D]`) specs are silently skipped:
//! they're either gaps awaiting implementation or intentionally
//! parked, so absence of a citation is the expected state for both.
//!
//! Only citations classified as `CitationKind::Test` count toward
//! coverage. Production-code citations satisfy the *traceability*
//! requirement (handled by `ReverseOrphanCheck`) but not the
//! *test-coverage* requirement this check enforces.

use std::collections::BTreeSet;

use crate::LidRepo;
use crate::model::{CitationKind, SpecId, SpecStatus};

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct CoverageCheck;

impl Check for CoverageCheck {
    fn id(&self) -> CheckId {
        CheckId::Coverage
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let tested: BTreeSet<&SpecId> = repo
            .citations
            .iter()
            .filter(|c| c.kind == CitationKind::Test)
            .map(|c| &c.id)
            .collect();

        let mut findings = Vec::new();
        for spec_file in &repo.specs {
            for line in &spec_file.specs {
                if line.status != SpecStatus::Implemented {
                    continue;
                }
                if tested.contains(&line.id) {
                    continue;
                }
                findings.push(Finding {
                    check: CheckId::Coverage,
                    severity: Severity::Warning,
                    category: Category::Coverage,
                    message: format!(
                        "spec `{}` is marked [x] (implemented) but has no `@spec` citation in any test file",
                        line.id,
                    ),
                    location: Some(Location {
                        path: spec_file.path.clone(),
                        line: Some(line.line),
                    }),
                    spec: Some(line.id.clone()),
                    remediation: Some(format!(
                        "add `@spec {}` to a test that exercises this requirement",
                        line.id
                    )),
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
    use crate::model::{ArrowIndex, SpecCitation, SpecFile, SpecLine, SpecStatus, Unmapped};

    fn spec_line(id: &str, status: SpecStatus, line: usize) -> SpecLine {
        SpecLine {
            id: SpecId::parse(id).unwrap(),
            status,
            text: "stub".into(),
            line,
        }
    }

    fn cite(id: &str, kind: CitationKind, line: usize) -> SpecCitation {
        SpecCitation {
            id: SpecId::parse(id).unwrap(),
            file: PathBuf::from(format!("file-{line}.rs")),
            line,
            kind,
        }
    }

    fn make_repo(spec_lines: Vec<SpecLine>, citations: Vec<SpecCitation>) -> LidRepo {
        let root = PathBuf::from("/fake/root");
        let specs = vec![SpecFile {
            path: root.join("docs/specs/auth-specs.md"),
            specs: spec_lines,
            implementing_artifacts: vec![],
            lld: None,
            prefix: None,
        }];
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
            decision_docs: vec![],
        }
    }

    #[test]
    fn implemented_spec_with_test_citation_is_silent() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Implemented, 5)],
            vec![cite("AUTH-001", CitationKind::Test, 12)],
        );
        let f = CoverageCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn implemented_spec_without_any_citation_is_flagged() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Implemented, 5)],
            vec![],
        );
        let f = CoverageCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warning);
        assert_eq!(f[0].category, Category::Coverage);
        assert!(f[0].message.contains("AUTH-001"));
    }

    #[test]
    fn code_only_citation_does_not_count_as_coverage() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Implemented, 5)],
            vec![cite("AUTH-001", CitationKind::Code, 12)],
        );
        let f = CoverageCheck.run(&repo);
        // The spec is cited in code but not in a test — still flagged
        // because the methodology asks for *test* coverage.
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].spec.as_ref().unwrap().as_str(), "AUTH-001");
    }

    #[test]
    fn open_spec_is_not_flagged() {
        let repo = make_repo(vec![spec_line("AUTH-001", SpecStatus::Open, 5)], vec![]);
        let f = CoverageCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn deferred_spec_is_not_flagged() {
        let repo = make_repo(vec![spec_line("AUTH-001", SpecStatus::Deferred, 5)], vec![]);
        let f = CoverageCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn each_uncovered_spec_gets_its_own_finding() {
        let repo = make_repo(
            vec![
                spec_line("AUTH-001", SpecStatus::Implemented, 5),
                spec_line("AUTH-002", SpecStatus::Implemented, 6),
                spec_line("AUTH-003", SpecStatus::Implemented, 7),
            ],
            vec![cite("AUTH-002", CitationKind::Test, 1)],
        );
        let f = CoverageCheck.run(&repo);
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn finding_location_points_at_spec_line() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Implemented, 42)],
            vec![],
        );
        let f = CoverageCheck.run(&repo);
        let loc = f[0].location.as_ref().unwrap();
        assert_eq!(loc.line, Some(42));
        assert!(loc.path.ends_with("auth-specs.md"));
    }

    #[test]
    fn check_id_is_coverage() {
        assert_eq!(CoverageCheck.id(), CheckId::Coverage);
    }
}
