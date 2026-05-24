//! Spec-status / citation drift check.
//!
//! Surfaces inconsistencies between a spec's declared status marker
//! and what the source actually shows. Specifically:
//!
//! * A spec marked `[ ]` (open / active gap) but cited from source —
//!   the work appears to have landed without the marker being flipped
//!   to `[x]`.
//! * A spec marked `[D]` (deferred) but cited from source — either
//!   the spec was implemented anyway or the citation is leftover code
//!   that should be removed.
//!
//! The complementary direction — `[x]` with no citation — is the
//! province of `CoverageCheck`; together the two checks catch both
//! sides of the marker-vs-reality split. Findings ship at Warning
//! severity because the right fix needs human judgement (was the
//! spec really implemented, or is the citation stale?).

use std::collections::BTreeMap;

use crate::LidRepo;
use crate::model::{SpecCitation, SpecId, SpecStatus};

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct SpecStatusCountsCheck;

impl Check for SpecStatusCountsCheck {
    fn id(&self) -> CheckId {
        CheckId::SpecStatusCounts
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let cited: BTreeMap<&SpecId, &SpecCitation> =
            repo.citations.iter().map(|c| (&c.id, c)).collect();

        let mut findings = Vec::new();
        for spec_file in &repo.specs {
            for line in &spec_file.specs {
                let Some(_first_citation) = cited.get(&line.id) else {
                    continue;
                };
                match line.status {
                    SpecStatus::Open => {
                        findings.push(drift_finding(
                            spec_file.path.clone(),
                            line.line,
                            line.id.clone(),
                            format!(
                                "spec `{}` is marked [ ] (open) but is cited by `@spec` in source — looks implemented",
                                line.id
                            ),
                            "flip the marker to `[x]` if the work is done, or remove the stale citation".to_owned(),
                        ));
                    }
                    SpecStatus::Deferred => {
                        findings.push(drift_finding(
                            spec_file.path.clone(),
                            line.line,
                            line.id.clone(),
                            format!(
                                "spec `{}` is marked [D] (deferred) but is cited by `@spec` in source",
                                line.id
                            ),
                            "if the spec is actually implemented, flip the marker to `[x]`; otherwise remove the citation"
                                .to_owned(),
                        ));
                    }
                    SpecStatus::Implemented => {}
                }
            }
        }
        findings
    }
}

fn drift_finding(
    path: std::path::PathBuf,
    line: usize,
    spec: SpecId,
    message: String,
    remediation: String,
) -> Finding {
    Finding {
        check: CheckId::SpecStatusCounts,
        severity: Severity::Warning,
        category: Category::Coverage,
        message,
        location: Some(Location {
            path,
            line: Some(line),
        }),
        spec: Some(spec),
        remediation: Some(remediation),
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

    fn spec_line(id: &str, status: SpecStatus, line: usize) -> SpecLine {
        SpecLine {
            id: SpecId::parse(id).unwrap(),
            status,
            text: "stub".into(),
            line,
        }
    }

    fn cite(id: &str, kind: CitationKind) -> SpecCitation {
        SpecCitation {
            id: SpecId::parse(id).unwrap(),
            file: PathBuf::from("src/auth.rs"),
            line: 7,
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
        }
    }

    #[test]
    fn implemented_spec_with_citation_is_silent() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Implemented, 5)],
            vec![cite("AUTH-001", CitationKind::Code)],
        );
        let f = SpecStatusCountsCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn open_spec_without_citation_is_silent() {
        let repo = make_repo(vec![spec_line("AUTH-001", SpecStatus::Open, 5)], vec![]);
        let f = SpecStatusCountsCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn open_spec_with_citation_is_flagged() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Open, 5)],
            vec![cite("AUTH-001", CitationKind::Code)],
        );
        let f = SpecStatusCountsCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warning);
        assert!(f[0].message.contains("[ ] (open)"));
        assert!(f[0].message.contains("AUTH-001"));
    }

    #[test]
    fn deferred_spec_with_citation_is_flagged() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Deferred, 5)],
            vec![cite("AUTH-001", CitationKind::Test)],
        );
        let f = SpecStatusCountsCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("[D] (deferred)"));
    }

    #[test]
    fn deferred_spec_without_citation_is_silent() {
        let repo = make_repo(vec![spec_line("AUTH-001", SpecStatus::Deferred, 5)], vec![]);
        let f = SpecStatusCountsCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn finding_location_points_at_spec_line() {
        let repo = make_repo(
            vec![spec_line("AUTH-001", SpecStatus::Open, 42)],
            vec![cite("AUTH-001", CitationKind::Code)],
        );
        let f = SpecStatusCountsCheck.run(&repo);
        let loc = f[0].location.as_ref().unwrap();
        assert_eq!(loc.line, Some(42));
        assert!(loc.path.ends_with("auth-specs.md"));
    }

    #[test]
    fn multiple_specs_each_drifting_each_get_finding() {
        let repo = make_repo(
            vec![
                spec_line("AUTH-001", SpecStatus::Open, 5),
                spec_line("AUTH-002", SpecStatus::Deferred, 6),
            ],
            vec![
                cite("AUTH-001", CitationKind::Code),
                cite("AUTH-002", CitationKind::Test),
            ],
        );
        let f = SpecStatusCountsCheck.run(&repo);
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn check_id_is_spec_status_counts() {
        assert_eq!(SpecStatusCountsCheck.id(), CheckId::SpecStatusCounts);
    }
}
