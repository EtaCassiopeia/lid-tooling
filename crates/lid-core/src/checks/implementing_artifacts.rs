//! `implementing_artifacts` check.
//!
//! Verifies that every path listed under `implementing_artifacts` in a
//! spec file (`*-specs.md`) actually exists on disk. Stale references
//! silently accumulate after files are renamed or deleted; this check
//! surfaces them as `Error` findings so they get fixed.

use std::path::Path;

use crate::LidRepo;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct ImplementingArtifactsCheck;

impl Check for ImplementingArtifactsCheck {
    fn id(&self) -> CheckId {
        CheckId::ImplementingArtifacts
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let mut findings = Vec::new();
        for spec_file in &repo.specs {
            for artifact in &spec_file.implementing_artifacts {
                let abs = if artifact.is_absolute() {
                    artifact.clone()
                } else {
                    repo.root.join(artifact)
                };
                if abs.exists() {
                    continue;
                }
                findings.push(Finding {
                    check: CheckId::ImplementingArtifacts,
                    severity: Severity::Error,
                    category: Category::References,
                    message: format!(
                        "`{}` declares implementing artifact `{}` which does not exist",
                        display_relative(repo, &spec_file.path),
                        artifact.display()
                    ),
                    location: Some(Location {
                        path: spec_file.path.clone(),
                        line: None,
                    }),
                    spec: None,
                    remediation: Some(
                        "create the referenced file or remove it from implementing_artifacts"
                            .to_owned(),
                    ),
                });
            }
        }
        findings
    }
}

fn display_relative(repo: &LidRepo, path: &Path) -> String {
    path.strip_prefix(&repo.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::model::{ArrowIndex, SpecFile, Unmapped};
    use crate::repo::LidRepo;

    fn make_repo(root: PathBuf, spec_files: Vec<SpecFile>) -> LidRepo {
        LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 2,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs: spec_files,
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
        }
    }

    #[test]
    fn missing_artifact_produces_error_finding() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let repo = make_repo(
            root.clone(),
            vec![SpecFile {
                path: root.join("docs/intent/auth/auth-specs.md"),
                specs: vec![],
                implementing_artifacts: vec![PathBuf::from("src/auth.rs")],
                lld: None,
                prefix: None,
            }],
        );

        let findings = ImplementingArtifactsCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].check, CheckId::ImplementingArtifacts);
        assert!(
            findings[0].message.contains("src/auth.rs"),
            "got: {}",
            findings[0].message
        );
    }

    #[test]
    fn existing_artifact_produces_no_finding() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/auth.rs"), "// stub\n").unwrap();

        let repo = make_repo(
            root.clone(),
            vec![SpecFile {
                path: root.join("docs/intent/auth/auth-specs.md"),
                specs: vec![],
                implementing_artifacts: vec![PathBuf::from("src/auth.rs")],
                lld: None,
                prefix: None,
            }],
        );

        let findings = ImplementingArtifactsCheck.run(&repo);
        assert!(
            findings.is_empty(),
            "expected no findings, got {findings:?}"
        );
    }

    #[test]
    fn no_implementing_artifacts_produces_no_finding() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let repo = make_repo(
            root.clone(),
            vec![SpecFile {
                path: root.join("docs/intent/auth/auth-specs.md"),
                specs: vec![],
                implementing_artifacts: vec![],
                lld: None,
                prefix: None,
            }],
        );

        let findings = ImplementingArtifactsCheck.run(&repo);
        assert!(findings.is_empty());
    }

    #[test]
    fn absolute_artifact_path_is_checked_directly() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let abs_path = root.join("src/auth.rs");

        let repo = make_repo(
            root.clone(),
            vec![SpecFile {
                path: root.join("docs/intent/auth/auth-specs.md"),
                specs: vec![],
                implementing_artifacts: vec![abs_path.clone()],
                lld: None,
                prefix: None,
            }],
        );

        let findings = ImplementingArtifactsCheck.run(&repo);
        assert_eq!(findings.len(), 1, "absolute missing path should still flag");
    }

    #[test]
    fn multiple_artifacts_each_checked_independently() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/exists.rs"), "// stub\n").unwrap();

        let repo = make_repo(
            root.clone(),
            vec![SpecFile {
                path: root.join("docs/intent/auth/auth-specs.md"),
                specs: vec![],
                implementing_artifacts: vec![
                    PathBuf::from("src/exists.rs"),
                    PathBuf::from("src/missing.rs"),
                ],
                lld: None,
                prefix: None,
            }],
        );

        let findings = ImplementingArtifactsCheck.run(&repo);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("src/missing.rs"));
    }
}
