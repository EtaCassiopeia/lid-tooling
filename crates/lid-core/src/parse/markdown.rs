//! Markdown parsers for LID artifacts.
//!
//! The methodology's structured artifacts (EARS spec files, arrow docs,
//! LLDs) all live in markdown with conventional patterns rather than a
//! formal grammar. This module uses tight line-level regexes for the
//! patterns it consumes; we reach for `pulldown-cmark` only when we need
//! to walk tables or section trees (which is for the arrow-doc and LLD
//! parsers in later commits, not for the spec-line parser here).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::error::{LidError, Result};
use crate::model::{SpecFile, SpecId, SpecLine, SpecStatus};

/// Matches `- [x] **AUTH-001**: text` and its `[ ]`/`[D]` variants.
///
/// The captured ID shape mirrors the anchored pattern in `SpecId::parse`
/// (uppercase start, optional uppercase/digit body, then one-or-more
/// `-SEGMENT` groups). The digit-presence post-filter still runs inside
/// `SpecId::parse`.
#[allow(clippy::expect_used)] // constant pattern; compilation is infallible
static SPEC_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^- \[([ xD])\] \*\*([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)\*\*: (.+)$")
        .expect("SPEC_LINE_RE compiles")
});

/// Matches `**LLD**: docs/llds/...md` header line.
#[allow(clippy::expect_used)]
static LLD_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\*\*LLD\*\*:\s+(.+?)\s*$").expect("LLD_HEADER_RE compiles"));

/// Matches the `**Implementing artifacts**:` header line; the bullet list
/// follows on subsequent lines.
#[allow(clippy::expect_used)]
static IMPL_ARTIFACTS_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\*\*Implementing artifacts\*\*:\s*$").expect("IMPL_ARTIFACTS_HEADER_RE compiles")
});

/// Matches `- path/to/file` after stripping the marker.
#[allow(clippy::expect_used)]
static BULLET_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- (.+?)\s*$").expect("BULLET_LINE_RE compiles"));

/// Load and parse an EARS spec file.
///
/// # Errors
/// Returns [`LidError::Io`] when the file cannot be read, or
/// [`LidError::Markdown`] when a spec-shaped line carries an invalid ID.
pub fn load_spec_file(path: &Path) -> Result<SpecFile> {
    let content = fs::read_to_string(path).map_err(|source| LidError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_spec_file(&content, path)
}

/// Parse an EARS spec file from an in-memory string.
///
/// # Errors
/// Returns [`LidError::Markdown`] when a spec-shaped line carries an
/// invalid ID; `source_path` is included in the error message for
/// locatability and need not exist on disk.
pub fn parse_spec_file(content: &str, source_path: &Path) -> Result<SpecFile> {
    let mut specs: Vec<SpecLine> = Vec::new();
    let mut implementing_artifacts: Vec<PathBuf> = Vec::new();
    let mut lld: Option<PathBuf> = None;
    let mut in_artifacts_section = false;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;

        if let Some(caps) = SPEC_LINE_RE.captures(raw_line) {
            // Spec lines always close the `Implementing artifacts` bullet list.
            in_artifacts_section = false;

            let status = match &caps[1] {
                "x" => SpecStatus::Implemented,
                "D" => SpecStatus::Deferred,
                " " => SpecStatus::Open,
                // Regex character class forbids any other value.
                _ => unreachable!("SPEC_LINE_RE constrains status marker to [ xD]"),
            };

            let raw_id = &caps[2];
            let id = SpecId::parse(raw_id).map_err(|source| LidError::Markdown {
                path: source_path.to_path_buf(),
                line: line_no,
                source: Box::new(source),
            })?;

            specs.push(SpecLine {
                id,
                status,
                text: caps[3].to_owned(),
                line: line_no,
            });
            continue;
        }

        if let Some(caps) = LLD_HEADER_RE.captures(raw_line) {
            lld = Some(PathBuf::from(&caps[1]));
            in_artifacts_section = false;
            continue;
        }

        if IMPL_ARTIFACTS_HEADER_RE.is_match(raw_line) {
            in_artifacts_section = true;
            continue;
        }

        if in_artifacts_section {
            if let Some(caps) = BULLET_LINE_RE.captures(raw_line) {
                implementing_artifacts.push(PathBuf::from(&caps[1]));
                continue;
            }
            // First non-bullet, non-blank line ends the artifacts list.
            if !raw_line.trim().is_empty() {
                in_artifacts_section = false;
            }
        }
    }

    Ok(SpecFile {
        path: source_path.to_path_buf(),
        specs,
        implementing_artifacts,
        lld,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn parse(content: &str) -> Result<SpecFile> {
        parse_spec_file(content, Path::new("<inline>.md"))
    }

    /// Mirrors the upstream `docs/specs/arrow-maintenance-specs.md` header
    /// + body shape.
    const UPSTREAM_SHAPED: &str = "\
# arrow-maintenance command-mode specs

**LLD**: docs/llds/arrow-maintenance.md
**Implementing artifacts**:
- plugins/arrow-maintenance/skills/arrow-maintenance/SKILL.md
- plugins/arrow-maintenance/skills/arrow-maintenance/references/index-schema.md

**Scope**: These specs cover the command-mode behavior.

Status markers: `[x]` implemented · `[ ]` active gap · `[D]` deferred

---

## Invocation Dispatch

- [x] **ARROW-MAINT-001**: When the user invokes `/arrow-maintenance`, the system SHALL run an audit-and-update pass.
- [ ] **ARROW-MAINT-002**: Future behavior not yet implemented.
- [D] **ARROW-MAINT-003**: Deferred for a later milestone.
";

    #[test]
    fn parses_status_markers() {
        let f = parse(UPSTREAM_SHAPED).unwrap();
        assert_eq!(f.specs.len(), 3);
        assert_eq!(f.specs[0].status, SpecStatus::Implemented);
        assert_eq!(f.specs[1].status, SpecStatus::Open);
        assert_eq!(f.specs[2].status, SpecStatus::Deferred);
    }

    #[test]
    fn captures_ids_and_text() {
        let f = parse(UPSTREAM_SHAPED).unwrap();
        assert_eq!(f.specs[0].id.as_str(), "ARROW-MAINT-001");
        assert!(f.specs[0].text.starts_with("When the user invokes"));
    }

    #[test]
    fn captures_line_numbers() {
        let f = parse(UPSTREAM_SHAPED).unwrap();
        // The first spec is at line 16 (1-indexed); the marker block lives above.
        assert_eq!(f.specs[0].line, 16);
        assert_eq!(f.specs[1].line, 17);
        assert_eq!(f.specs[2].line, 18);
    }

    #[test]
    fn extracts_lld_header() {
        let f = parse(UPSTREAM_SHAPED).unwrap();
        assert_eq!(
            f.lld.as_deref(),
            Some(Path::new("docs/llds/arrow-maintenance.md"))
        );
    }

    #[test]
    fn extracts_implementing_artifacts() {
        let f = parse(UPSTREAM_SHAPED).unwrap();
        assert_eq!(f.implementing_artifacts.len(), 2);
        assert_eq!(
            f.implementing_artifacts[0],
            PathBuf::from("plugins/arrow-maintenance/skills/arrow-maintenance/SKILL.md")
        );
    }

    #[test]
    fn empty_file_returns_empty_spec_file() {
        let f = parse("").unwrap();
        assert!(f.specs.is_empty());
        assert!(f.implementing_artifacts.is_empty());
        assert!(f.lld.is_none());
    }

    #[test]
    fn file_with_no_lld_or_artifacts_still_parses() {
        let content = "\
# Just specs

- [x] **AUTH-001**: text.
- [ ] **AUTH-002**: more text.
";
        let f = parse(content).unwrap();
        assert_eq!(f.specs.len(), 2);
        assert!(f.lld.is_none());
        assert!(f.implementing_artifacts.is_empty());
    }

    #[test]
    fn spec_section_breaks_artifact_capture() {
        // Verifies that a spec line after the artifact list correctly
        // closes the in-artifacts state (no spurious paths captured).
        let content = "\
**Implementing artifacts**:
- a.md
- b.md

## Body

- [x] **AUTH-001**: text.
";
        let f = parse(content).unwrap();
        assert_eq!(f.implementing_artifacts.len(), 2);
        assert_eq!(f.specs.len(), 1);
    }

    #[test]
    fn artifact_capture_ends_on_non_bullet_line() {
        let content = "\
**Implementing artifacts**:
- a.md
some prose
- not-an-artifact.md
";
        let f = parse(content).unwrap();
        assert_eq!(f.implementing_artifacts.len(), 1);
        assert_eq!(f.implementing_artifacts[0], PathBuf::from("a.md"));
    }

    #[test]
    fn malformed_spec_id_returns_markdown_error_with_line() {
        // `A-Z` matches the shape regex but fails the digit post-check.
        let content = "\
# Title

- [x] **A-Z**: invalid because no digit.
";
        let err = parse(content).unwrap_err();
        match err {
            LidError::Markdown { line, .. } => assert_eq!(line, 3),
            other => panic!("expected Markdown error, got {other:?}"),
        }
    }

    #[test]
    fn ignores_non_spec_bullets() {
        let content = "\
- Just a regular bullet, not a spec.
- [x] Also not a spec (no bold id).
- [x] **DONE**: still not a spec (id lacks a hyphen).
";
        let f = parse(content).unwrap();
        assert!(f.specs.is_empty());
    }

    #[test]
    fn handles_real_upstream_specs_file() {
        // Smoke test: parse the actual arrow-maintenance specs from the
        // upstream LID repo (vendored only as a small inline snippet).
        let content = "\
**LLD**: docs/llds/arrow-maintenance.md
**Implementing artifacts**:
- plugins/arrow-maintenance/skills/arrow-maintenance/SKILL.md

- [x] **ARROW-MAINT-001**: When the user invokes `/arrow-maintenance` on a project where `docs/arrows/` is present, the system SHALL run an audit-and-update pass.
- [x] **ARROW-MAINT-018**: When an audit detects a candidate split or merge, the system SHALL surface the candidate as a finding.
";
        let f = parse(content).unwrap();
        assert_eq!(f.specs.len(), 2);
        assert_eq!(f.specs[0].id.as_str(), "ARROW-MAINT-001");
        assert_eq!(f.specs[1].id.as_str(), "ARROW-MAINT-018");
        assert_eq!(f.implementing_artifacts.len(), 1);
        assert!(f.lld.is_some());
    }
}
