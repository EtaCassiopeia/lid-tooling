//! Project discovery and aggregate state for a LID-shaped repository.
//!
//! Given a starting path, walks upward to find the nearest
//! `docs/arrows/index.yaml`, then loads every artifact the coherence
//! checks consult: the index, every spec file under `docs/intent/`
//! (matched by `*-specs.md`), every design doc under `docs/intent/`
//! (matched by `*-design.md`), every arrow detail doc under `docs/arrows/`,
//! and every `@spec` citation in source files reachable from the root.
//!
//! Requires `schema_version: 2` in `docs/arrows/index.yaml` — the layout
//! introduced in LID v1.2.0. Projects on earlier schema versions must
//! migrate before use; `discover` returns `LidError::UnsupportedSchemaVersion`
//! so callers never receive a partially-loaded repo from an unknown layout.
//!
//! All stored paths are absolute (after canonicalising the discovered
//! root). Presenting them as repo-relative is a concern of the CLI / LSP
//! adapters, not of the model.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{LidError, Result};
use crate::model::{ArrowDoc, ArrowIndex, LldDoc, SpecCitation, SpecFile};
use crate::parse;

/// Schema versions this build of lid-tooling can parse correctly.
///
/// Minimum supported: 2 (LID v1.2.0, `docs/intent/` tree layout).
/// When a new LID schema ships, add its version here alongside the new loader.
const SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[2];

/// Aggregate state for one LID-shaped project.
#[derive(Debug, Clone)]
pub struct LidRepo {
    /// Absolute, canonicalised path to the project root — the parent of
    /// `docs/arrows/index.yaml`.
    pub root: PathBuf,
    pub index: ArrowIndex,
    pub specs: Vec<SpecFile>,
    pub llds: Vec<LldDoc>,
    pub arrow_docs: Vec<ArrowDoc>,
    pub citations: Vec<SpecCitation>,
}

impl LidRepo {
    /// Discover and load a LID repository.
    ///
    /// Walks upward from `start` looking for `docs/arrows/index.yaml`, then
    /// loads every artifact the coherence checks need. Stops at the first
    /// match; does not search siblings.
    ///
    /// # Errors
    /// - [`LidError::NotALidRepo`] when no `docs/arrows/index.yaml` is
    ///   reachable above `start`.
    /// - [`LidError::Io`] when a discovered file cannot be read.
    /// - [`LidError::Yaml`] or [`LidError::Markdown`] when a discovered
    ///   file is malformed.
    pub fn discover(start: &Path) -> Result<Self> {
        let root = find_root(start)?;
        let index_path = root.join("docs").join("arrows").join("index.yaml");
        let index = parse::yaml::load_from_path(&index_path)?;

        if !SUPPORTED_SCHEMA_VERSIONS.contains(&index.schema_version) {
            return Err(LidError::UnsupportedSchemaVersion {
                found: index.schema_version,
                supported: SUPPORTED_SCHEMA_VERSIONS,
            });
        }

        let specs = load_md_tree(
            &root.join("docs").join("intent"),
            "-specs.md",
            parse::markdown::load_spec_file,
        )?;
        let llds = load_md_tree(
            &root.join("docs").join("intent"),
            "-design.md",
            parse::markdown::load_lld,
        )?;
        let arrow_docs = load_md_tree(
            &root.join("docs").join("arrows"),
            ".md",
            parse::markdown::load_arrow_doc,
        )?;

        let citations = parse::source::scan_path(&root);

        Ok(Self {
            root,
            index,
            specs,
            llds,
            arrow_docs,
            citations,
        })
    }
}

/// Walk upward from `start` looking for the nearest directory that
/// contains `docs/arrows/index.yaml`. Returns the absolute path to that
/// directory (the LID project root).
fn find_root(start: &Path) -> Result<PathBuf> {
    let absolute = fs::canonicalize(start).map_err(|source| LidError::Io {
        path: start.to_path_buf(),
        source,
    })?;
    let mut current = absolute.as_path();
    loop {
        if current
            .join("docs")
            .join("arrows")
            .join("index.yaml")
            .is_file()
        {
            return Ok(current.to_path_buf());
        }
        let Some(parent) = current.parent() else {
            return Err(LidError::NotALidRepo { start: absolute });
        };
        current = parent;
    }
}

/// Recursively walk `dir`, loading every `*.md` file whose name ends with
/// `suffix` (pass `".md"` to match all markdown files). Skips README.md and
/// `_`-prefixed files. Results are sorted by path for deterministic ordering.
fn load_md_tree<T, F>(dir: &Path, suffix: &str, load: F) -> Result<Vec<T>>
where
    F: Fn(&Path) -> Result<T>,
{
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_md_paths(dir, suffix, &mut paths)?;
    paths.sort();
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        out.push(load(&p)?);
    }
    Ok(out)
}

fn collect_md_paths(dir: &Path, suffix: &str, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| LidError::Io {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| LidError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_md_paths(&path, suffix, out)?;
        } else if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("md")
            && !is_non_artifact_filename(&path)
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(suffix))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn is_non_artifact_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.eq_ignore_ascii_case("README.md") || name.starts_with('_')
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// Build a minimal schema v2 LID-shaped tempdir for tests.
    ///
    /// Layout:
    /// ```text
    /// {root}/
    ///   docs/
    ///     arrows/
    ///       index.yaml        (schema_version: 2)
    ///       auth.md
    ///       README.md         (skipped)
    ///     intent/
    ///       auth/
    ///         auth-specs.md
    ///         auth-design.md
    ///         _template.md    (skipped)
    ///   src/
    ///     auth.rs (with @spec citations)
    /// ```
    fn build_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs/arrows")).unwrap();
        fs::create_dir_all(root.join("docs/intent/auth")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();

        fs::write(
            root.join("docs/arrows/index.yaml"),
            "\
schema_version: 2
arrows:
  auth:
    status: MAPPED
    detail: auth.md
    blocks: []
    blockedBy: []
",
        )
        .unwrap();

        fs::write(
            root.join("docs/arrows/auth.md"),
            "\
# Arrow: auth

## References

### LLD
- docs/intent/auth/auth-design.md

### EARS
- docs/intent/auth/auth-specs.md
",
        )
        .unwrap();

        // README.md should be ignored; _template.md should be ignored.
        fs::write(root.join("docs/arrows/README.md"), "directory README").unwrap();
        fs::write(root.join("docs/intent/auth/_template.md"), "template").unwrap();

        fs::write(
            root.join("docs/intent/auth/auth-specs.md"),
            "\
# auth specs

- [x] **AUTH-001**: When the user logs in, the system SHALL ...
- [ ] **AUTH-002**: Another requirement.
",
        )
        .unwrap();

        fs::write(
            root.join("docs/intent/auth/auth-design.md"),
            "\
# Design: auth

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| Session store | Redis | Memcached | Latency |
",
        )
        .unwrap();

        fs::write(
            root.join("src/auth.rs"),
            "// @spec AUTH-001\nfn login() {}\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn discover_finds_root_at_starting_path() {
        let repo_dir = build_repo();
        let repo = LidRepo::discover(repo_dir.path()).unwrap();
        let canonical_root = fs::canonicalize(repo_dir.path()).unwrap();
        assert_eq!(repo.root, canonical_root);
    }

    #[test]
    fn discover_walks_upward_from_subdirectory() {
        let repo_dir = build_repo();
        let start = repo_dir.path().join("src");
        let repo = LidRepo::discover(&start).unwrap();
        let canonical_root = fs::canonicalize(repo_dir.path()).unwrap();
        assert_eq!(repo.root, canonical_root);
    }

    #[test]
    fn discover_fails_when_no_lid_layout_present() {
        let dir = tempfile::tempdir().unwrap();
        let err = LidRepo::discover(dir.path()).unwrap_err();
        assert!(matches!(err, LidError::NotALidRepo { .. }), "got {err:?}");
    }

    #[test]
    fn discover_loads_all_artifact_categories() {
        let repo_dir = build_repo();
        let repo = LidRepo::discover(repo_dir.path()).unwrap();

        assert_eq!(repo.index.arrows.len(), 1);
        assert_eq!(repo.specs.len(), 1, "specs: {:?}", repo.specs);
        assert_eq!(repo.specs[0].specs.len(), 2);
        assert_eq!(repo.llds.len(), 1);
        assert_eq!(repo.llds[0].decisions.len(), 1);
        assert_eq!(
            repo.arrow_docs.len(),
            1,
            "arrow_docs: {:?}",
            repo.arrow_docs
        );
    }

    #[test]
    fn discover_skips_readme_and_underscore_files() {
        let repo_dir = build_repo();
        let repo = LidRepo::discover(repo_dir.path()).unwrap();
        // README.md sits in docs/arrows/ but must not be loaded as an arrow doc.
        assert!(
            repo.arrow_docs
                .iter()
                .all(|d| !d.path.ends_with("README.md")),
            "README.md should have been skipped: {:?}",
            repo.arrow_docs
        );
        assert!(
            repo.specs.iter().all(|s| !s.path.ends_with("_template.md")),
            "_template.md should have been skipped"
        );
    }

    #[test]
    fn discover_collects_source_citations() {
        let repo_dir = build_repo();
        let repo = LidRepo::discover(repo_dir.path()).unwrap();
        assert!(
            repo.citations.iter().any(|c| c.id.as_str() == "AUTH-001"),
            "expected AUTH-001 citation, got {:?}",
            repo.citations
        );
    }

    #[test]
    fn discover_propagates_yaml_errors() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs/arrows")).unwrap();
        fs::write(
            root.join("docs/arrows/index.yaml"),
            "schema_version: nope\n",
        )
        .unwrap();
        let err = LidRepo::discover(root).unwrap_err();
        assert!(matches!(err, LidError::Yaml { .. }), "got {err:?}");
    }

    #[test]
    fn discover_rejects_unsupported_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs/arrows")).unwrap();
        fs::write(
            root.join("docs/arrows/index.yaml"),
            "schema_version: 1\narrows: {}\n",
        )
        .unwrap();
        let err = LidRepo::discover(root).unwrap_err();
        assert!(
            matches!(err, LidError::UnsupportedSchemaVersion { found: 1, .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn discover_propagates_markdown_errors() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs/arrows")).unwrap();
        fs::create_dir_all(root.join("docs/intent/bad")).unwrap();
        fs::write(
            root.join("docs/arrows/index.yaml"),
            "schema_version: 2\narrows: {}\n",
        )
        .unwrap();
        // Spec line with a no-digit ID — must fail parsing.
        fs::write(
            root.join("docs/intent/bad/bad-specs.md"),
            "- [x] **A-Z**: missing digit\n",
        )
        .unwrap();
        let err = LidRepo::discover(root).unwrap_err();
        assert!(matches!(err, LidError::Markdown { .. }), "got {err:?}");
    }
}
