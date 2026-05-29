//! Arrow-doc structure check.
//!
//! Verifies that every `### {Kind}` subsection inside `## References` uses
//! one of the recognised category names. Unrecognised names — typically
//! caused by typos (`### Ears` instead of `### EARS`) or casing errors
//! (`### lld`) — result in bullets being silently dropped by the parser,
//! which then makes reference-coherence and orphan checks miss those files.
//!
//! Severity: `Info`. These findings are visible as hints in the IDE/LSP
//! without blocking CI unless `--fail-on info` is set.

use crate::LidRepo;
use crate::model::KNOWN_REFERENCE_SECTIONS;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct ArrowDocStructureCheck;

impl Check for ArrowDocStructureCheck {
    fn id(&self) -> CheckId {
        CheckId::ArrowDocStructure
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let mut findings = Vec::new();
        for doc in &repo.arrow_docs {
            for section in &doc.unrecognized_reference_sections {
                let rel = doc
                    .path
                    .strip_prefix(&repo.root)
                    .unwrap_or(&doc.path)
                    .display()
                    .to_string();
                findings.push(Finding {
                    check: CheckId::ArrowDocStructure,
                    severity: Severity::Info,
                    category: Category::References,
                    message: format!(
                        "arrow doc `{rel}` has unrecognised `## References` subsection `### {section}`"
                    ),
                    location: Some(Location {
                        path: doc.path.clone(),
                        line: None,
                    }),
                    spec: None,
                    remediation: Some(format!(
                        "rename `### {section}` to one of the known categories: {}",
                        KNOWN_REFERENCE_SECTIONS.join(", ")
                    )),
                });
            }
        }
        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::model::{
        ArrowDoc, ArrowIndex, ArrowReferences, Segment, SegmentId, Status, Unmapped,
    };

    fn make_repo(unrecognized: Vec<String>) -> LidRepo {
        let root = PathBuf::from("/fake/root");
        let doc = ArrowDoc {
            path: root.join("docs/arrows/auth.md"),
            references: ArrowReferences::default(),
            unrecognized_reference_sections: unrecognized,
        };
        let mut arrows = BTreeMap::new();
        arrows.insert(
            SegmentId::parse("auth").unwrap(),
            Segment {
                status: Status::Mapped,
                sampled: None,
                audited: None,
                audited_sha: None,
                blocks: vec![],
                blocked_by: vec![],
                detail: PathBuf::from("auth.md"),
                next: None,
                drift: None,
                merged_into: None,
                children: vec![],
                parent: None,
            },
        );
        LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 2,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows,
                unmapped: Unmapped::default(),
            },
            specs: vec![],
            llds: vec![],
            arrow_docs: vec![doc],
            citations: vec![],
        }
    }

    #[test]
    fn check_id_is_arrow_doc_structure() {
        assert_eq!(ArrowDocStructureCheck.id(), CheckId::ArrowDocStructure);
    }

    #[test]
    fn no_unrecognised_sections_yields_no_findings() {
        let repo = make_repo(vec![]);
        assert!(ArrowDocStructureCheck.run(&repo).is_empty());
    }

    #[test]
    fn each_unrecognised_section_gets_a_finding() {
        let repo = make_repo(vec!["Ears".into(), "lld".into()]);
        let findings = ArrowDocStructureCheck.run(&repo);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn finding_names_the_bad_section_and_lists_valid_ones() {
        let repo = make_repo(vec!["Ears".into()]);
        let f = &ArrowDocStructureCheck.run(&repo)[0];
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(f.category, Category::References);
        assert!(f.message.contains("Ears"));
        let rem = f.remediation.as_deref().unwrap();
        assert!(rem.contains("EARS"));
        assert!(rem.contains("LLD"));
    }
}
