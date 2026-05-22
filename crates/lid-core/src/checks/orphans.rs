//! Orphan-artifact check.
//!
//! Implements §5 of the upstream audit-checklist. Walks the loaded LLDs
//! and spec files and flags any whose path is not referenced from any
//! arrow doc's `## References` section. These artifacts exist on disk
//! but aren't attached to the arrow of intent — they need to be either
//! linked to a segment or deleted.
//!
//! Scope: this check covers LLD and spec orphans only. Source-file
//! orphans (`@spec` annotations in code that aren't cited from any
//! arrow doc) belong to a future check; reverse orphans (`@spec`
//! references to non-existent specs) are a separate check.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::LidRepo;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct OrphanCheck;

impl Check for OrphanCheck {
    fn id(&self) -> CheckId {
        CheckId::Orphan
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let referenced_llds = collect_referenced_paths(repo, |refs| &refs.lld);
        let referenced_specs = collect_referenced_paths(repo, |refs| &refs.ears);
        let unmapped_llds = unmapped_paths(repo, |u| &u.docs.llds);
        let unmapped_specs = unmapped_paths(repo, |u| &u.docs.specs);

        let mut findings = Vec::new();
        for lld in &repo.llds {
            if referenced_llds.contains(&lld.path) || unmapped_llds.contains(&lld.path) {
                continue;
            }
            findings.push(orphan_finding(repo, &lld.path, "LLD"));
        }
        for spec in &repo.specs {
            if referenced_specs.contains(&spec.path) || unmapped_specs.contains(&spec.path) {
                continue;
            }
            findings.push(orphan_finding(repo, &spec.path, "spec"));
        }
        findings
    }
}

fn collect_referenced_paths<F>(repo: &LidRepo, select: F) -> BTreeSet<PathBuf>
where
    F: Fn(&crate::model::ArrowReferences) -> &Vec<String>,
{
    repo.arrow_docs
        .iter()
        .flat_map(|d| select(&d.references).iter())
        .filter_map(|r| resolve_ref(repo, r))
        .collect()
}

fn unmapped_paths<F>(repo: &LidRepo, select: F) -> BTreeSet<PathBuf>
where
    F: Fn(&crate::model::Unmapped) -> &Vec<PathBuf>,
{
    select(&repo.index.unmapped)
        .iter()
        .map(|p| {
            if p.is_absolute() {
                p.clone()
            } else {
                repo.root.join(p)
            }
        })
        .collect()
}

fn resolve_ref(repo: &LidRepo, raw: &str) -> Option<PathBuf> {
    let path_part = raw.split('§').next().unwrap_or(raw).trim();
    if path_part.is_empty() {
        return None;
    }
    let p = Path::new(path_part);
    Some(if p.is_absolute() {
        p.to_path_buf()
    } else {
        repo.root.join(p)
    })
}

fn orphan_finding(repo: &LidRepo, path: &Path, kind: &str) -> Finding {
    let rel = path
        .strip_prefix(&repo.root)
        .unwrap_or(path)
        .display()
        .to_string();
    Finding {
        check: CheckId::Orphan,
        severity: Severity::Warning,
        category: Category::Orphans,
        message: format!("{kind} `{rel}` is not referenced from any arrow doc"),
        location: Some(Location {
            path: path.to_path_buf(),
            line: None,
        }),
        spec: None,
        remediation: Some(format!(
            "link `{rel}` from an arrow doc's `## References`, add it to `unmapped.docs`, or delete it"
        )),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::model::{
        ArrowDoc, ArrowIndex, ArrowReferences, DecisionRow, LldDoc, Segment, SegmentId, SpecFile,
        Status, Unmapped, UnmappedDocs,
    };

    fn seg_id(s: &str) -> SegmentId {
        SegmentId::parse(s).unwrap()
    }

    fn minimal_segment(detail: &str) -> Segment {
        Segment {
            status: Status::Mapped,
            sampled: None,
            audited: None,
            audited_sha: None,
            blocks: vec![],
            blocked_by: vec![],
            detail: PathBuf::from(detail),
            next: None,
            drift: None,
            merged_into: None,
        }
    }

    fn make_repo(
        lld_files: &[&str],
        spec_files: &[&str],
        arrow_refs: ArrowReferences,
        unmapped: Unmapped,
    ) -> LidRepo {
        let root = PathBuf::from("/fake/root");

        let llds = lld_files
            .iter()
            .map(|f| LldDoc {
                path: root.join("docs/llds").join(f),
                decisions: vec![DecisionRow {
                    decision: "x".into(),
                    chosen: "y".into(),
                    alternatives: "z".into(),
                    rationale: "ok".into(),
                    inferred: false,
                }],
            })
            .collect();
        let specs = spec_files
            .iter()
            .map(|f| SpecFile {
                path: root.join("docs/specs").join(f),
                specs: vec![],
                implementing_artifacts: vec![],
                lld: None,
            })
            .collect();

        let arrow_doc = ArrowDoc {
            path: root.join("docs/arrows/auth.md"),
            references: arrow_refs,
        };
        let mut arrows = BTreeMap::new();
        arrows.insert(seg_id("auth"), minimal_segment("auth.md"));

        LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows,
                unmapped,
            },
            specs,
            llds,
            arrow_docs: vec![arrow_doc],
            citations: vec![],
        }
    }

    #[test]
    fn lld_referenced_from_arrow_is_not_orphaned() {
        let refs = ArrowReferences {
            lld: vec!["docs/llds/auth.md".into()],
            ..Default::default()
        };
        let repo = make_repo(&["auth.md"], &[], refs, Unmapped::default());
        let findings = OrphanCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn lld_not_referenced_is_orphaned() {
        let repo = make_repo(
            &["lonely.md"],
            &[],
            ArrowReferences::default(),
            Unmapped::default(),
        );
        let findings = OrphanCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("LLD"));
        assert!(findings[0].message.contains("lonely.md"));
    }

    #[test]
    fn spec_not_referenced_is_orphaned() {
        let repo = make_repo(
            &[],
            &["lonely-specs.md"],
            ArrowReferences::default(),
            Unmapped::default(),
        );
        let findings = OrphanCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("spec"));
        assert!(findings[0].message.contains("lonely-specs.md"));
    }

    #[test]
    fn lld_in_unmapped_docs_is_not_orphaned() {
        let unmapped = Unmapped {
            docs: UnmappedDocs {
                llds: vec![PathBuf::from("docs/llds/parked.md")],
                specs: vec![],
            },
        };
        let repo = make_repo(&["parked.md"], &[], ArrowReferences::default(), unmapped);
        let findings = OrphanCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn lld_orphan_uses_warning_severity() {
        let repo = make_repo(
            &["lonely.md"],
            &[],
            ArrowReferences::default(),
            Unmapped::default(),
        );
        let findings = OrphanCheck.run(&repo);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].category, Category::Orphans);
    }

    #[test]
    fn multiple_orphans_each_get_a_finding() {
        let repo = make_repo(
            &["a.md", "b.md"],
            &["c.md"],
            ArrowReferences::default(),
            Unmapped::default(),
        );
        let findings = OrphanCheck.run(&repo);
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn check_id_is_orphan() {
        assert_eq!(OrphanCheck.id(), CheckId::Orphan);
    }
}
