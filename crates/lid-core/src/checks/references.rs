//! Reference coherence check.
//!
//! Walks every arrow doc's `## References` section and verifies that
//! each bullet points at a file that actually exists. Implements check
//! §1 from `arrow-maintenance/references/audit-checklist.md`.
//!
//! Bullets are stored as raw strings (see `model::arrow_doc`). For HLD
//! bullets the methodology permits a `path.md §Section` form, so this
//! check splits on the first `§` and validates only the path part —
//! section-anchor resolution would require an HLD parser and is
//! deferred.

use std::path::Path;

use crate::LidRepo;
use crate::model::ArrowDoc;

use super::{Category, Check, CheckId, Finding, Location, Severity, resolve_arrow_ref};

pub struct ReferenceCoherenceCheck;

impl Check for ReferenceCoherenceCheck {
    fn id(&self) -> CheckId {
        CheckId::ReferenceCoherence
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let mut findings = Vec::new();
        for arrow_doc in &repo.arrow_docs {
            check_category(
                repo,
                arrow_doc,
                "HLD",
                &arrow_doc.references.hld,
                &mut findings,
            );
            check_category(
                repo,
                arrow_doc,
                "LLD",
                &arrow_doc.references.lld,
                &mut findings,
            );
            check_category(
                repo,
                arrow_doc,
                "EARS",
                &arrow_doc.references.ears,
                &mut findings,
            );
            check_category(
                repo,
                arrow_doc,
                "Tests",
                &arrow_doc.references.tests,
                &mut findings,
            );
            check_category(
                repo,
                arrow_doc,
                "Code",
                &arrow_doc.references.code,
                &mut findings,
            );
        }
        findings
    }
}

fn check_category(
    repo: &LidRepo,
    arrow_doc: &ArrowDoc,
    category: &str,
    bullets: &[String],
    findings: &mut Vec<Finding>,
) {
    for raw in bullets {
        let Some(resolved) = resolve_arrow_ref(repo, raw) else {
            continue;
        };
        if !resolved.exists() {
            findings.push(Finding {
                check: CheckId::ReferenceCoherence,
                severity: Severity::Error,
                category: Category::References,
                message: format!(
                    "arrow `{}` cites missing {category} reference `{raw}`",
                    display_relative(repo, &arrow_doc.path),
                ),
                location: Some(Location {
                    path: arrow_doc.path.clone(),
                    line: None,
                }),
                spec: None,
                remediation: Some(format!(
                    "ensure `{}` exists or update the bullet in `## References / ### {category}`",
                    display_relative(repo, &resolved),
                )),
            });
        }
    }
}

fn display_relative(repo: &LidRepo, path: &Path) -> String {
    path.strip_prefix(&repo.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::model::{ArrowIndex, ArrowReferences, Segment, SegmentId, Status, Unmapped};

    fn seg_id(s: &str) -> SegmentId {
        SegmentId::parse(s).unwrap()
    }

    /// Build a tempdir with the touched files and an in-memory `LidRepo`
    /// whose `arrow_docs[0]` carries the given references.
    fn make_repo(files_to_create: &[&str], refs: ArrowReferences) -> (tempfile::TempDir, LidRepo) {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();

        for rel in files_to_create {
            let abs = root.join(rel);
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&abs, "stub").unwrap();
        }

        let arrow_doc_path = root.join("docs/arrows/auth.md");
        let arrow_doc = ArrowDoc {
            path: arrow_doc_path,
            references: refs,
            unrecognized_reference_sections: vec![],
        };

        let mut arrows = BTreeMap::new();
        arrows.insert(
            seg_id("auth"),
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
            llds: vec![],
            arrow_docs: vec![arrow_doc],
            citations: vec![],
        };
        (dir, repo)
    }

    #[test]
    fn all_present_references_yield_no_findings() {
        let refs = ArrowReferences {
            hld: vec!["docs/high-level-design.md §Auth".into()],
            lld: vec!["docs/intent/auth/auth-design.md".into()],
            ears: vec!["docs/intent/auth/auth-specs.md".into()],
            tests: vec!["tests/auth.test.ts".into()],
            code: vec!["src/auth.ts".into()],
        };
        let (_dir, repo) = make_repo(
            &[
                "docs/high-level-design.md",
                "docs/intent/auth/auth-design.md",
                "docs/intent/auth/auth-specs.md",
                "tests/auth.test.ts",
                "src/auth.ts",
            ],
            refs,
        );
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn flags_missing_lld_reference() {
        let refs = ArrowReferences {
            lld: vec!["docs/intent/auth/missing.md".into()],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&[], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("LLD"));
        assert!(findings[0].message.contains("missing.md"));
    }

    #[test]
    fn strips_section_anchor_from_hld_bullet() {
        let refs = ArrowReferences {
            hld: vec!["docs/high-level-design.md §Glossary".into()],
            ..Default::default()
        };
        // Create the file but with no validation of the anchor — it should pass.
        let (_dir, repo) = make_repo(&["docs/high-level-design.md"], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn flags_one_finding_per_missing_reference() {
        let refs = ArrowReferences {
            ears: vec![
                "docs/intent/auth/missing-a.md".into(),
                "docs/intent/auth/missing-b.md".into(),
            ],
            code: vec!["src/missing-c.ts".into()],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&[], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn empty_or_whitespace_bullets_are_skipped() {
        let refs = ArrowReferences {
            lld: vec![String::new(), "   ".into()],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&[], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn check_id_is_reference_coherence() {
        assert_eq!(ReferenceCoherenceCheck.id(), CheckId::ReferenceCoherence);
    }

    #[test]
    fn extracts_path_from_backtick_wrapped_bullet() {
        let refs = ArrowReferences {
            lld: vec!["`docs/intent/auth/auth-design.md`".into()],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&["docs/intent/auth/auth-design.md"], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn extracts_path_from_bullet_with_em_dash_trailer() {
        let refs = ArrowReferences {
            lld: vec!["`docs/intent/auth/auth-design.md` — this segment's LLD.".into()],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&["docs/intent/auth/auth-design.md"], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn extracts_path_from_bullet_with_multiple_section_anchors() {
        let refs = ArrowReferences {
            hld: vec![
                "`docs/high-level-design.md` § Architecture / Plugins; § Key Design Decisions"
                    .into(),
            ],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&["docs/high-level-design.md"], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn extracts_path_when_nested_code_span_follows() {
        // `path.md` (12 specs, prefix `AUTH-*`)
        let refs = ArrowReferences {
            ears: vec!["`docs/intent/auth/auth-specs.md` (12 specs, prefix `AUTH-*`)".into()],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&["docs/intent/auth/auth-specs.md"], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn directory_reference_passes_when_directory_exists() {
        use crate::model::{ArrowIndex, Segment, SegmentId, Status, Unmapped};
        use std::collections::BTreeMap;
        use std::path::PathBuf;
        // Bullets pointing at directories (workspace outputs, skill dirs, site
        // source trees) are valid as long as the directory exists.
        let refs = ArrowReferences {
            tests: vec!["`plugins/my-skill/workspace/iteration-1/` — eval outputs".into()],
            ..Default::default()
        };
        // Create the directory (no file inside needed).
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("plugins/my-skill/workspace/iteration-1")).unwrap();
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
            llds: vec![],
            arrow_docs: vec![ArrowDoc {
                path: PathBuf::from("docs/arrows/auth.md"),
                references: refs,
                unrecognized_reference_sections: vec![],
            }],
            citations: vec![],
        };
        let findings = ReferenceCoherenceCheck.run(&repo);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn prose_only_bullet_is_silently_skipped() {
        let refs = ArrowReferences {
            ears: vec!["None on the container itself. See the LLD for details.".into()],
            ..Default::default()
        };
        let (_dir, repo) = make_repo(&[], refs);
        let findings = ReferenceCoherenceCheck.run(&repo);
        // Prose without a path-shaped token is informational, not a missing ref.
        assert!(findings.is_empty(), "got {findings:?}");
    }
}
