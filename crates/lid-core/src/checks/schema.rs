//! Schema-level coherence check for `docs/arrows/index.yaml`.
//!
//! Most field-level constraints (status enum values, date formats, SHA
//! shape) are enforced at deserialization time via the model types in
//! `crate::model::arrow`. What's left for the schema check is the
//! *cross-reference* layer — relations that span entries:
//!
//! * Every segment named in `blocks` / `blockedBy` / `merged_into` must
//!   resolve to a key in `arrows`.
//! * `status == MERGED` requires `merged_into` to be present;
//!   `merged_into` present with a different status is suspicious.
//! * Every segment's `detail:` filename must correspond to a loaded
//!   arrow doc.
//! * Every segment listed under `taxonomy.{cluster}` must exist as a
//!   key in `arrows`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::LidRepo;
use crate::model::{SegmentId, Status};

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct SchemaCheck;

impl Check for SchemaCheck {
    fn id(&self) -> CheckId {
        CheckId::Schema
    }

    #[allow(clippy::too_many_lines)] // The body is a flat list of specific cross-checks; splitting into helpers spreads the same logic across many tiny functions.
    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let known: BTreeSet<&SegmentId> = repo.index.arrows.keys().collect();
        let arrows_dir = repo.root.join("docs").join("arrows");
        let known_arrow_doc_files: BTreeSet<PathBuf> = repo
            .arrow_docs
            .iter()
            .filter_map(|d| {
                d.path
                    .strip_prefix(&arrows_dir)
                    .ok()
                    .map(std::path::Path::to_path_buf)
            })
            .collect();
        let index_yaml_path = arrows_dir.join("index.yaml");

        let mut findings = Vec::new();

        for (seg_id, seg) in &repo.index.arrows {
            for blocked in &seg.blocks {
                if !known.contains(blocked) {
                    findings.push(error_at(
                        format!("segment `{seg_id}` blocks unknown segment `{blocked}`"),
                        &index_yaml_path,
                        Some(format!(
                            "remove `{blocked}` from `{seg_id}.blocks` or add it under `arrows`"
                        )),
                    ));
                }
            }
            for blocker in &seg.blocked_by {
                if !known.contains(blocker) {
                    findings.push(error_at(
                        format!("segment `{seg_id}` is blockedBy unknown segment `{blocker}`"),
                        &index_yaml_path,
                        Some(format!(
                            "remove `{blocker}` from `{seg_id}.blockedBy` or add it under `arrows`"
                        )),
                    ));
                }
            }

            match (&seg.merged_into, seg.status) {
                (Some(target), Status::Merged) if !known.contains(target) => {
                    findings.push(error_at(
                        format!("segment `{seg_id}` merged into unknown segment `{target}`"),
                        &index_yaml_path,
                        Some(format!("ensure `{target}` exists under `arrows`")),
                    ));
                }
                (Some(target), other_status) => {
                    findings.push(warning_at(
                        format!(
                            "segment `{seg_id}` declares `merged_into: {target}` but status is `{other_status:?}`, not MERGED"
                        ),
                        &index_yaml_path,
                        Some(
                            "set status to MERGED or drop the merged_into field".to_owned(),
                        ),
                    ));
                }
                (None, Status::Merged) => {
                    findings.push(error_at(
                        format!("segment `{seg_id}` has status MERGED but no `merged_into` target"),
                        &index_yaml_path,
                        Some("add `merged_into: <target-segment>` or change the status".to_owned()),
                    ));
                }
                (None, _) => {}
            }

            if !known_arrow_doc_files.contains(&seg.detail) {
                findings.push(error_at(
                    format!(
                        "segment `{seg_id}` detail file `{}` not found under `docs/arrows/`",
                        seg.detail.display()
                    ),
                    &arrows_dir.join(&seg.detail),
                    Some(format!(
                        "create `docs/arrows/{}` or update `{seg_id}.detail`",
                        seg.detail.display()
                    )),
                ));
            }
        }

        for (cluster, listed) in &repo.index.taxonomy {
            for seg in listed {
                if !known.contains(seg) {
                    findings.push(error_at(
                        format!("taxonomy cluster `{cluster}` references unknown segment `{seg}`"),
                        &index_yaml_path,
                        Some(format!(
                            "remove `{seg}` from `taxonomy.{cluster}` or add the segment"
                        )),
                    ));
                }
            }
        }

        findings
    }
}

fn finding_at(
    severity: Severity,
    message: String,
    path: &std::path::Path,
    remediation: Option<String>,
) -> Finding {
    Finding {
        check: CheckId::Schema,
        severity,
        category: Category::Schema,
        message,
        location: Some(Location {
            path: path.to_path_buf(),
            line: None,
        }),
        spec: None,
        remediation,
    }
}

fn error_at(message: String, path: &std::path::Path, remediation: Option<String>) -> Finding {
    finding_at(Severity::Error, message, path, remediation)
}

fn warning_at(message: String, path: &std::path::Path, remediation: Option<String>) -> Finding {
    finding_at(Severity::Warning, message, path, remediation)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::model::{ArrowDoc, ArrowIndex, ArrowReferences, Segment, SegmentId};

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

    fn repo_with(
        arrows: BTreeMap<SegmentId, Segment>,
        taxonomy: BTreeMap<String, Vec<SegmentId>>,
        arrow_doc_files: &[&str],
    ) -> LidRepo {
        let root = PathBuf::from("/fake/root");
        let arrows_dir = root.join("docs").join("arrows");
        let arrow_docs = arrow_doc_files
            .iter()
            .map(|f| ArrowDoc {
                path: arrows_dir.join(f),
                references: ArrowReferences::default(),
            })
            .collect();
        LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy,
                arrows,
                unmapped: crate::model::Unmapped::default(),
            },
            specs: vec![],
            llds: vec![],
            arrow_docs,
            citations: vec![],
        }
    }

    #[test]
    fn clean_index_yields_no_findings() {
        let mut arrows = BTreeMap::new();
        arrows.insert(seg_id("auth"), minimal_segment("auth.md"));
        let repo = repo_with(arrows, BTreeMap::new(), &["auth.md"]);
        let f = SchemaCheck.run(&repo);
        assert!(f.is_empty(), "expected no findings, got {f:?}");
    }

    #[test]
    fn flags_blocks_pointing_at_unknown_segment() {
        let mut arrows = BTreeMap::new();
        let mut auth = minimal_segment("auth.md");
        auth.blocks = vec![seg_id("unknown")];
        arrows.insert(seg_id("auth"), auth);
        let repo = repo_with(arrows, BTreeMap::new(), &["auth.md"]);
        let f = SchemaCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Error);
        assert!(f[0].message.contains("unknown segment `unknown`"));
    }

    #[test]
    fn flags_blocked_by_pointing_at_unknown_segment() {
        let mut arrows = BTreeMap::new();
        let mut auth = minimal_segment("auth.md");
        auth.blocked_by = vec![seg_id("ghost")];
        arrows.insert(seg_id("auth"), auth);
        let repo = repo_with(arrows, BTreeMap::new(), &["auth.md"]);
        let f = SchemaCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("blockedBy"));
    }

    #[test]
    fn flags_missing_detail_file() {
        let mut arrows = BTreeMap::new();
        arrows.insert(seg_id("auth"), minimal_segment("auth.md"));
        // arrow_doc_files is empty — auth.md isn't loaded.
        let repo = repo_with(arrows, BTreeMap::new(), &[]);
        let f = SchemaCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("detail file"));
        assert!(f[0].message.contains("auth.md"));
    }

    #[test]
    fn flags_merged_status_without_target() {
        let mut arrows = BTreeMap::new();
        let mut s = minimal_segment("auth.md");
        s.status = Status::Merged;
        s.merged_into = None;
        arrows.insert(seg_id("auth"), s);
        let repo = repo_with(arrows, BTreeMap::new(), &["auth.md"]);
        let f = SchemaCheck.run(&repo);
        assert!(f.iter().any(|x| x.message.contains("status MERGED but no")));
    }

    #[test]
    fn flags_merged_into_with_unknown_target() {
        let mut arrows = BTreeMap::new();
        let mut s = minimal_segment("auth.md");
        s.status = Status::Merged;
        s.merged_into = Some(seg_id("vanished"));
        arrows.insert(seg_id("auth"), s);
        let repo = repo_with(arrows, BTreeMap::new(), &["auth.md"]);
        let f = SchemaCheck.run(&repo);
        assert!(
            f.iter()
                .any(|x| x.message.contains("merged into unknown segment `vanished`"))
        );
    }

    #[test]
    fn warns_on_merged_into_without_merged_status() {
        let mut arrows = BTreeMap::new();
        let mut s = minimal_segment("auth.md");
        s.status = Status::Mapped;
        s.merged_into = Some(seg_id("session"));
        arrows.insert(seg_id("auth"), s);
        arrows.insert(seg_id("session"), minimal_segment("session.md"));
        let repo = repo_with(arrows, BTreeMap::new(), &["auth.md", "session.md"]);
        let f = SchemaCheck.run(&repo);
        assert!(
            f.iter()
                .any(|x| x.severity == Severity::Warning && x.message.contains("merged_into"))
        );
    }

    #[test]
    fn flags_taxonomy_referencing_unknown_segment() {
        let mut arrows = BTreeMap::new();
        arrows.insert(seg_id("auth"), minimal_segment("auth.md"));
        let taxonomy = BTreeMap::from([(
            "core".to_owned(),
            vec![seg_id("auth"), seg_id("nonexistent")],
        )]);
        let repo = repo_with(arrows, taxonomy, &["auth.md"]);
        let f = SchemaCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert!(
            f[0].message
                .contains("taxonomy cluster `core` references unknown segment `nonexistent`")
        );
    }

    #[test]
    fn check_id_is_schema() {
        assert_eq!(SchemaCheck.id(), CheckId::Schema);
    }
}
