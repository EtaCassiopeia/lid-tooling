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

use super::{Category, Check, CheckId, Finding, Location, Severity, resolve_arrow_ref};

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
        // schema v2: unmapped.docs.intent lists intent-tree files not yet attached to a segment.
        let unmapped_intent = unmapped_paths(repo, |u| &u.docs.intent);
        // schema v2 sub-HLD pattern: a segment whose detail: points to ../intent/... has no
        // arrow doc; the design doc is directly referenced via the detail field, so it must
        // not be flagged as an orphan.
        let detail_llds = collect_detail_llds(repo);

        let mut findings = Vec::new();
        for lld in &repo.llds {
            if referenced_llds.contains(&lld.path)
                || unmapped_llds.contains(&lld.path)
                || unmapped_intent.contains(&lld.path)
                || detail_llds.contains(&lld.path)
            {
                continue;
            }
            findings.push(orphan_finding(repo, &lld.path, "LLD"));
        }
        for spec in &repo.specs {
            if referenced_specs.contains(&spec.path)
                || unmapped_specs.contains(&spec.path)
                || unmapped_intent.contains(&spec.path)
            {
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
        .filter_map(|r| resolve_arrow_ref(repo, r))
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

/// Collect the absolute paths of design docs that are the `detail:` target of
/// any arrow segment. Used to exempt sub-HLD design docs from the orphan check:
/// in the schema v2 node-as-folder layout a sub-HLD has no arrow doc of its own
/// — its `detail` field points directly to `../intent/…` — so the design doc
/// will never appear in any `## References` section but is still reachable.
fn collect_detail_llds(repo: &LidRepo) -> BTreeSet<PathBuf> {
    let arrows_dir = repo.root.join("docs").join("arrows");
    repo.index
        .arrows
        .values()
        .map(|seg| normalize_path(&arrows_dir.join(&seg.detail)))
        .collect()
}

/// Resolve `..` and `.` components in `p` without touching the filesystem.
fn normalize_path(p: &Path) -> PathBuf {
    let mut components: Vec<std::path::Component<'_>> = Vec::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
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
            children: vec![],
            parent: None,
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
                path: root.join("docs/intent/auth").join(f),
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
                path: root.join("docs/intent/auth").join(f),
                specs: vec![],
                implementing_artifacts: vec![],
                lld: None,
                prefix: None,
            })
            .collect();

        let arrow_doc = ArrowDoc {
            path: root.join("docs/arrows/auth.md"),
            references: arrow_refs,
            unrecognized_reference_sections: vec![],
        };
        let mut arrows = BTreeMap::new();
        arrows.insert(seg_id("auth"), minimal_segment("auth.md"));

        LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 2,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows,
                unmapped,
            },
            specs,
            llds,
            arrow_docs: vec![arrow_doc],
            citations: vec![],
            decision_docs: vec![],
        }
    }

    #[test]
    fn lld_referenced_from_arrow_is_not_orphaned() {
        let refs = ArrowReferences {
            lld: vec!["docs/intent/auth/auth.md".into()],
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
                llds: vec![PathBuf::from("docs/intent/auth/parked.md")],
                specs: vec![],
                intent: vec![],
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

    /// Schema v2 sub-HLD pattern: a segment with `children` has no arrow doc;
    /// its `detail` points to `../intent/…`. The design doc must not be flagged.
    #[test]
    fn sub_hld_design_doc_referenced_via_detail_is_not_orphaned() {
        let root = PathBuf::from("/fake/root");
        // The sub-HLD segment's detail traverses out of docs/arrows/ into docs/intent/.
        let mut seg = minimal_segment("../intent/storage/storage-design.md");
        seg.children = vec![seg_id("store-interface")];
        let child = minimal_segment("storage/store-interface.md");

        let mut arrows = BTreeMap::new();
        arrows.insert(seg_id("storage"), seg);
        arrows.insert(seg_id("store-interface"), child);

        // The design doc is loaded as an LLD (matches *-design.md).
        let design_doc_path = root.join("docs/intent/storage/storage-design.md");
        let llds = vec![LldDoc {
            path: design_doc_path,
            decisions: vec![],
        }];

        let repo = LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 2,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows,
                unmapped: Unmapped::default(),
            },
            specs: vec![],
            llds,
            arrow_docs: vec![],
            citations: vec![],
            decision_docs: vec![],
        };

        let findings = OrphanCheck.run(&repo);
        assert!(
            findings.is_empty(),
            "sub-HLD design doc should not be orphaned, got {findings:?}"
        );
    }

    #[test]
    fn unmapped_intent_exempts_design_doc_from_orphan() {
        let unmapped = Unmapped {
            docs: UnmappedDocs {
                llds: vec![],
                specs: vec![],
                intent: vec![PathBuf::from("docs/intent/auth/parked-design.md")],
            },
        };
        let repo = make_repo(
            &["parked-design.md"],
            &[],
            ArrowReferences::default(),
            unmapped,
        );
        let findings = OrphanCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }
}
