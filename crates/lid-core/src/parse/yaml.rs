//! Parse `docs/arrows/index.yaml` into an [`ArrowIndex`].
//!
//! The schema is fixed by `model::arrow`; this module only adds the
//! `fs::read_to_string` boilerplate and a single conversion point from
//! `serde_yaml_ng` errors into [`LidError`].

use std::fs;
use std::path::Path;

use crate::error::{LidError, Result};
use crate::model::ArrowIndex;

/// Load an [`ArrowIndex`] from a YAML file on disk.
///
/// # Errors
/// Returns [`LidError::Io`] when the file cannot be read, or
/// [`LidError::Yaml`] when the contents do not match the schema.
pub fn load_from_path(path: &Path) -> Result<ArrowIndex> {
    let content = fs::read_to_string(path).map_err(|source| LidError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    load_from_str(&content, path)
}

/// Parse an [`ArrowIndex`] from an in-memory YAML string. The `source_path`
/// is only used to make error messages locatable; it doesn't need to exist
/// on disk.
///
/// # Errors
/// Returns [`LidError::Yaml`] when the contents do not match the schema.
pub fn load_from_str(content: &str, source_path: &Path) -> Result<ArrowIndex> {
    serde_yaml_ng::from_str(content).map_err(|source| LidError::Yaml {
        path: source_path.to_path_buf(),
        source: Box::new(source),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::{SegmentId, Status};

    fn parse(content: &str) -> Result<ArrowIndex> {
        load_from_str(content, &PathBuf::from("<inline>"))
    }

    /// Minimal valid YAML — exercises the path where most fields default.
    const MINIMAL: &str = r"
schema_version: 1
arrows: {}
";

    /// Mirrors the upstream LID repository's `docs/arrows/index.yaml`
    /// shape (subset). Asserts the cross-module wiring works end to end.
    const UPSTREAM_SHAPED: &str = r"
schema_version: 1
last_updated: 2026-05-18

taxonomy:
  core:
    - linked-intent-dev
    - arrow-maintenance

arrows:
  linked-intent-dev:
    status: MAPPED
    sampled: 2026-05-05
    audited: null
    audited_sha: null
    blocks: [arrow-maintenance]
    blockedBy: []
    detail: linked-intent-dev.md
    next: null
    drift: null

  arrow-maintenance:
    status: MAPPED
    sampled: 2026-04-25
    blocks: []
    blockedBy: [linked-intent-dev]
    detail: arrow-maintenance.md

unmapped:
  docs:
    llds: []
    specs: []
";

    fn seg(id: &str) -> SegmentId {
        SegmentId::parse(id).unwrap()
    }

    #[test]
    fn parses_minimal_yaml() {
        let idx = parse(MINIMAL).unwrap();
        assert_eq!(idx.schema_version, 1);
        assert!(idx.arrows.is_empty());
        assert!(idx.taxonomy.is_empty());
    }

    #[test]
    fn parses_upstream_shaped_yaml() {
        let idx = parse(UPSTREAM_SHAPED).unwrap();
        assert_eq!(idx.schema_version, 1);
        assert_eq!(idx.arrows.len(), 2);

        let lid_dev = idx.arrows.get(&seg("linked-intent-dev")).unwrap();
        assert_eq!(lid_dev.status, Status::Mapped);
        assert_eq!(lid_dev.blocks, vec![seg("arrow-maintenance")]);
        assert!(lid_dev.blocked_by.is_empty());
        assert_eq!(lid_dev.detail.to_string_lossy(), "linked-intent-dev.md");
        assert!(lid_dev.audited.is_none());
        assert!(lid_dev.audited_sha.is_none());

        let am = idx.arrows.get(&seg("arrow-maintenance")).unwrap();
        assert_eq!(am.blocked_by, vec![seg("linked-intent-dev")]);

        assert_eq!(
            idx.taxonomy.get("core"),
            Some(&vec![seg("linked-intent-dev"), seg("arrow-maintenance")])
        );
    }

    #[test]
    fn invalid_status_returns_yaml_error() {
        let bad = r"
schema_version: 1
arrows:
  auth:
    status: BOGUS
    detail: auth.md
";
        let err = parse(bad).unwrap_err();
        assert!(matches!(err, LidError::Yaml { .. }), "got {err:?}");
    }

    #[test]
    fn invalid_segment_name_in_blocks_returns_yaml_error() {
        let bad = r"
schema_version: 1
arrows:
  auth:
    status: MAPPED
    blocks: [Capital-Letters]
    detail: auth.md
";
        let err = parse(bad).unwrap_err();
        assert!(matches!(err, LidError::Yaml { .. }), "got {err:?}");
    }

    #[test]
    fn missing_required_field_returns_yaml_error() {
        // No `schema_version`.
        let bad = r"
arrows: {}
";
        let err = parse(bad).unwrap_err();
        assert!(matches!(err, LidError::Yaml { .. }), "got {err:?}");
    }

    #[test]
    fn io_error_when_file_missing() {
        let path = PathBuf::from("/definitely/does/not/exist.yaml");
        let err = load_from_path(&path).unwrap_err();
        assert!(matches!(err, LidError::Io { .. }), "got {err:?}");
    }
}
