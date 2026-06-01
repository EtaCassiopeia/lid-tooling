//! Markdown parsers for LID artifacts.
//!
//! The methodology's structured artifacts (EARS spec files, arrow docs,
//! LLDs) all live in Markdown with conventional patterns rather than a
//! formal grammar. The approach per artifact:
//!
//! - **EARS spec files** (`parse_spec_file`): line-level regex scanner.
//!   Spec lines require 1-based line numbers for LSP hover and the `[D]`
//!   deferred marker isn't a `CommonMark` task-list item, so staying at the
//!   line level is simpler and more precise.
//! - **Arrow detail docs** (`parse_arrow_doc`): `pulldown-cmark` event
//!   stream. Section trees (`## References` / `### LLD`) need structural
//!   heading detection that is fragile with prefix matching.
//! - **LLD design docs** (`parse_lld`): `pulldown-cmark` event stream with
//!   `ENABLE_TABLES`. GFM table parsing handles column alignment, escaped
//!   pipes in cells, and varying column counts correctly.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::error::{LidError, Result};
use crate::model::{
    ArrowDoc, ArrowReferences, DecisionDoc, DecisionRow, DecisionScope, KNOWN_REFERENCE_SECTIONS,
    LldDoc, SpecFile, SpecId, SpecLine, SpecStatus,
};

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

/// One spec-shaped line, parsed from a single source-file line. Used
/// by the LSP server to identify when the cursor is positioned on a
/// spec definition (`- [x] **AUTH-001**: …`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecLineMatch {
    pub id: SpecId,
    pub status: SpecStatus,
    pub text: String,
    /// Byte range of the spec ID within the source line (without the
    /// surrounding `**`).
    pub id_byte_range: std::ops::Range<usize>,
}

/// Parse a single line and return the spec it defines, if any.
///
/// Returns `None` when the line isn't spec-shaped or when the captured
/// ID fails `SpecId::parse` (e.g. `A-Z` has no digit). The handler
/// callers in `lid-lsp` use this to position-test the cursor against
/// the `**SPEC-ID**` span.
#[must_use]
pub fn find_spec_line_match(line: &str) -> Option<SpecLineMatch> {
    let caps = SPEC_LINE_RE.captures(line)?;
    let id_match = caps.get(2)?;
    let id = SpecId::parse(id_match.as_str()).ok()?;
    let status = match &caps[1] {
        "x" => SpecStatus::Implemented,
        "D" => SpecStatus::Deferred,
        " " => SpecStatus::Open,
        _ => return None,
    };
    Some(SpecLineMatch {
        id,
        status,
        text: caps[3].to_owned(),
        id_byte_range: id_match.start()..id_match.end(),
    })
}

/// Replace the status marker on the line that defines `spec_id`.
///
/// Returns the updated file content, or `None` if `spec_id` is not found.
#[must_use]
pub fn update_spec_status_in_text(
    content: &str,
    spec_id: &SpecId,
    new_status: SpecStatus,
) -> Option<String> {
    let marker = match new_status {
        SpecStatus::Implemented => 'x',
        SpecStatus::Open => ' ',
        SpecStatus::Deferred => 'D',
    };
    let target = spec_id.as_ref();
    let mut found = false;
    let updated = content
        .lines()
        .map(|line| {
            if !found {
                if let Some(m) = find_spec_line_match(line) {
                    if m.id.as_ref() == target {
                        found = true;
                        return format!("- [{marker}] **{target}**: {}", m.text);
                    }
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    if found {
        // Preserve trailing newline if original had one.
        if content.ends_with('\n') {
            Some(updated + "\n")
        } else {
            Some(updated)
        }
    } else {
        None
    }
}

/// Replace the text on the line that defines `spec_id`, preserving its status marker.
///
/// Returns the updated file content, or `None` if `spec_id` is not found.
#[must_use]
pub fn update_spec_text_in_text(content: &str, spec_id: &SpecId, new_text: &str) -> Option<String> {
    let target = spec_id.as_ref();
    let mut found = false;
    let updated = content
        .lines()
        .map(|line| {
            if !found {
                if let Some(m) = find_spec_line_match(line) {
                    if m.id.as_ref() == target {
                        found = true;
                        let marker = match m.status {
                            SpecStatus::Implemented => 'x',
                            SpecStatus::Open => ' ',
                            SpecStatus::Deferred => 'D',
                        };
                        return format!("- [{marker}] **{target}**: {new_text}");
                    }
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    if found {
        if content.ends_with('\n') {
            Some(updated + "\n")
        } else {
            Some(updated)
        }
    } else {
        None
    }
}

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
    let mut prefix: Option<String> = None;

    // YAML frontmatter state: detect `---` on the very first line.
    let mut first_line = true;
    let mut in_frontmatter = false;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;

        if first_line {
            first_line = false;
            if raw_line.trim() == "---" {
                in_frontmatter = true;
                continue;
            }
        }

        if in_frontmatter {
            if raw_line.trim() == "---" {
                in_frontmatter = false;
            } else if let Some(val) = raw_line.strip_prefix("prefix:") {
                prefix = Some(val.trim().to_owned());
            }
            continue;
        }

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
        prefix,
    })
}

/// Load and parse an arrow detail doc (`docs/arrows/{segment}.md`).
///
/// # Errors
/// Returns [`LidError::Io`] when the file cannot be read.
pub fn load_arrow_doc(path: &Path) -> Result<ArrowDoc> {
    let content = fs::read_to_string(path).map_err(|source| LidError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_arrow_doc(&content, path))
}

/// Parse an arrow detail doc from an in-memory string.
///
/// Captures bullets under `## References` grouped by `### {Kind}`
/// subheading (`HLD`, `LLD`, `EARS`, `Tests`, `Code`). Unknown subsection
/// names are ignored so that future additions to the methodology don't
/// fail parsing. Bullet text is collected from all inline events (plain
/// text, inline code) so that both `` `path.md` `` and `path.md §Section`
/// forms are preserved verbatim for later path resolution.
#[must_use]
pub fn parse_arrow_doc(content: &str, source_path: &Path) -> ArrowDoc {
    let mut refs = ArrowReferences::default();
    let mut unrecognized: Vec<String> = Vec::new();
    let mut in_references = false;
    let mut h3_name = String::new();
    let mut collecting_heading: Option<HeadingLevel> = None;
    let mut heading_buf = String::new();
    let mut in_item = false;
    let mut item_buf = String::new();

    for event in Parser::new(content) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                collecting_heading = Some(level);
                heading_buf.clear();
                in_item = false;
            }
            Event::End(TagEnd::Heading(level)) => {
                let text = heading_buf.trim().to_owned();
                collecting_heading = None;
                match level {
                    HeadingLevel::H2 => {
                        in_references = text == "References";
                        h3_name.clear();
                    }
                    HeadingLevel::H3 if in_references => {
                        if !KNOWN_REFERENCE_SECTIONS.contains(&text.as_str()) {
                            unrecognized.push(text.clone());
                        }
                        h3_name = text;
                    }
                    HeadingLevel::H3 => h3_name.clear(),
                    _ => {}
                }
            }
            Event::Text(t) | Event::Code(t) if collecting_heading.is_some() => {
                heading_buf.push_str(&t);
            }
            Event::Start(Tag::Item) if in_references => {
                in_item = true;
                item_buf.clear();
            }
            Event::End(TagEnd::Item) if in_item => {
                let bullet = item_buf.trim().to_owned();
                if !bullet.is_empty() {
                    match h3_name.as_str() {
                        "HLD" => refs.hld.push(bullet),
                        "LLD" => refs.lld.push(bullet),
                        "EARS" => refs.ears.push(bullet),
                        "Tests" => refs.tests.push(bullet),
                        "Code" => refs.code.push(bullet),
                        _ => {}
                    }
                }
                in_item = false;
            }
            Event::Text(t) if in_item => item_buf.push_str(&t),
            Event::Code(t) if in_item => {
                item_buf.push('`');
                item_buf.push_str(&t);
                item_buf.push('`');
            }
            Event::SoftBreak | Event::HardBreak if in_item => item_buf.push(' '),
            _ => {}
        }
    }

    ArrowDoc {
        path: source_path.to_path_buf(),
        references: refs,
        unrecognized_reference_sections: unrecognized,
    }
}

/// Load and parse a standalone decision document.
///
/// The `scope` must be determined by the caller from the file's location:
/// `DecisionScope::Project` for files under `docs/decisions/` and
/// `DecisionScope::Node { segment }` for files under
/// `docs/intent/<segment>/decisions/`.
///
/// # Errors
/// Returns [`LidError::Io`] when the file cannot be read.
pub fn load_decision_doc(path: &Path, scope: DecisionScope) -> Result<DecisionDoc> {
    let content = fs::read_to_string(path).map_err(|source| LidError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_decision_doc(&content, path, scope))
}

/// Parse a decision document from an in-memory string.
///
/// Extracts the first H1 heading as the title. Returns an empty title when
/// no `# Heading` line is present — the `decisions-structure` check flags
/// that case as a warning.
#[must_use]
pub fn parse_decision_doc(content: &str, source_path: &Path, scope: DecisionScope) -> DecisionDoc {
    let title = content
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .unwrap_or("")
        .trim()
        .to_owned();
    DecisionDoc {
        path: source_path.to_path_buf(),
        scope,
        title,
    }
}

/// Load and parse a Low-Level Design doc (`docs/llds/{name}.md`).
///
/// # Errors
/// Returns [`LidError::Io`] when the file cannot be read.
pub fn load_lld(path: &Path) -> Result<LldDoc> {
    let content = fs::read_to_string(path).map_err(|source| LidError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_lld(&content, path))
}

/// Parse an LLD from an in-memory string, extracting rows of the
/// `Decisions & Alternatives` table when present.
///
/// Uses `pulldown-cmark` with GFM table support so that escaped pipes
/// inside cells (`` `key: a \| b` ``) and alignment markers are handled
/// correctly. The header row (column names) and separator row are emitted
/// as a `TableHead` event and are skipped; only `TableRow` events
/// contribute `DecisionRow` values. The methodology's canonical shape is
/// four columns (Decision, Chosen, Alternatives, Rationale); any row with
/// fewer typed cells is padded with empty strings by the parser.
#[must_use]
pub fn parse_lld(content: &str, source_path: &Path) -> LldDoc {
    let mut decisions: Vec<DecisionRow> = Vec::new();
    let mut in_decisions_h2 = false;
    let mut collecting_h2 = false;
    let mut h2_buf = String::new();
    let mut in_table = false;
    let mut is_header_row = false;
    let mut current_row: Vec<String> = Vec::new();
    let mut in_cell = false;
    let mut cell_buf = String::new();

    for event in Parser::new_ext(content, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => {
                collecting_h2 = true;
                h2_buf.clear();
                in_table = false;
            }
            Event::Text(t) if collecting_h2 => h2_buf.push_str(&t),
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                collecting_h2 = false;
                in_decisions_h2 = matches!(
                    h2_buf.trim(),
                    "Decisions & Alternatives" | "Decisions and Alternatives"
                );
            }
            Event::Start(Tag::Table(_)) if in_decisions_h2 => in_table = true,
            Event::End(TagEnd::Table) => in_table = false,
            Event::Start(Tag::TableHead) => is_header_row = true,
            Event::End(TagEnd::TableHead) => is_header_row = false,
            Event::Start(Tag::TableRow) => current_row.clear(),
            Event::End(TagEnd::TableRow) if in_table && !is_header_row => {
                let inferred = current_row.iter().any(|c| c.contains("[inferred]"));
                decisions.push(DecisionRow {
                    decision: current_row.first().cloned().unwrap_or_default(),
                    chosen: current_row.get(1).cloned().unwrap_or_default(),
                    alternatives: current_row.get(2).cloned().unwrap_or_default(),
                    rationale: current_row.get(3).cloned().unwrap_or_default(),
                    inferred,
                });
                current_row.clear();
            }
            Event::Start(Tag::TableCell) if in_table => {
                in_cell = true;
                cell_buf.clear();
            }
            Event::End(TagEnd::TableCell) if in_table && !is_header_row => {
                current_row.push(std::mem::take(&mut cell_buf).trim().to_owned());
                in_cell = false;
            }
            Event::End(TagEnd::TableCell) if in_table => in_cell = false,
            Event::Text(t) | Event::Code(t) if in_cell && !is_header_row => {
                cell_buf.push_str(&t);
            }
            _ => {}
        }
    }

    LldDoc {
        path: source_path.to_path_buf(),
        decisions,
    }
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
    fn find_spec_line_match_extracts_id_with_byte_range() {
        let line = "- [x] **AUTH-001**: requirement text";
        let m = find_spec_line_match(line).unwrap();
        assert_eq!(m.id.as_str(), "AUTH-001");
        assert_eq!(m.status, SpecStatus::Implemented);
        assert_eq!(m.text, "requirement text");
        assert_eq!(&line[m.id_byte_range.clone()], "AUTH-001");
    }

    #[test]
    fn find_spec_line_match_handles_open_and_deferred_markers() {
        let open = find_spec_line_match("- [ ] **AUTH-002**: text").unwrap();
        assert_eq!(open.status, SpecStatus::Open);
        let deferred = find_spec_line_match("- [D] **AUTH-003**: text").unwrap();
        assert_eq!(deferred.status, SpecStatus::Deferred);
    }

    #[test]
    fn find_spec_line_match_returns_none_for_non_spec_lines() {
        assert!(find_spec_line_match("just prose").is_none());
        assert!(find_spec_line_match("- a regular bullet").is_none());
        assert!(find_spec_line_match("- [x] not a spec id").is_none());
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

    // ── Arrow doc parser ───────────────────────────────────────────────

    fn arrow(content: &str) -> ArrowDoc {
        parse_arrow_doc(content, Path::new("<inline>.md"))
    }

    const ARROW_DOC_SAMPLE: &str = "\
# Arrow: auth

## Status

**OK** — last audited 2026-04-01.

## References

### HLD

- docs/high-level-design.md §Authentication

### LLD

- docs/llds/auth.md

### EARS

- docs/specs/auth-specs.md

### Tests

- tests/auth/login.test.ts
- tests/auth/logout.test.ts

### Code

- src/auth/login.ts
- src/auth/logout.ts

## Spec Coverage

| Category | Spec IDs | Implemented |
| --- | --- | --- |
| auth | AUTH-001 to AUTH-010 | 10 |

## Key Findings

1. Coverage is complete.
";

    #[test]
    fn arrow_doc_collects_all_reference_categories() {
        let d = arrow(ARROW_DOC_SAMPLE);
        assert_eq!(d.references.hld.len(), 1);
        assert_eq!(d.references.lld, vec!["docs/llds/auth.md".to_owned()]);
        assert_eq!(
            d.references.ears,
            vec!["docs/specs/auth-specs.md".to_owned()]
        );
        assert_eq!(d.references.tests.len(), 2);
        assert_eq!(d.references.code.len(), 2);
    }

    #[test]
    fn arrow_doc_preserves_hld_section_anchor() {
        let d = arrow(ARROW_DOC_SAMPLE);
        assert_eq!(
            d.references.hld[0],
            "docs/high-level-design.md §Authentication"
        );
    }

    #[test]
    fn arrow_doc_with_empty_content_yields_empty_references() {
        let d = arrow("");
        assert!(d.references.hld.is_empty());
        assert!(d.references.lld.is_empty());
    }

    #[test]
    fn arrow_doc_ignores_bullets_outside_references_section() {
        let content = "\
## Status
- this is not a reference

## References

### LLD
- docs/llds/auth.md

## Spec Coverage
- also not a reference
";
        let d = arrow(content);
        assert_eq!(d.references.lld, vec!["docs/llds/auth.md".to_owned()]);
        assert!(d.references.hld.is_empty());
        assert!(d.references.tests.is_empty());
    }

    #[test]
    fn arrow_doc_ignores_unknown_subsections() {
        let content = "\
## References

### Future-category

- docs/something.md

### LLD

- docs/llds/auth.md
";
        let d = arrow(content);
        assert_eq!(d.references.lld.len(), 1);
        // Unknown subsection's bullets are silently dropped.
        assert!(d.references.hld.is_empty());
    }

    // ── LLD parser ─────────────────────────────────────────────────────

    fn lld(content: &str) -> LldDoc {
        parse_lld(content, Path::new("<inline>.md"))
    }

    #[test]
    fn lld_with_no_decisions_section_yields_empty() {
        let content = "\
# LLD: auth

## Context and Design Philosophy

Some prose.

## Plugin Structure

Some prose.
";
        let d = lld(content);
        assert!(d.decisions.is_empty());
    }

    #[test]
    fn lld_extracts_single_decision_row() {
        let content = "\
## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| Storage layer | PostgreSQL | SQLite, DynamoDB | Existing infra |
";
        let d = lld(content);
        assert_eq!(d.decisions.len(), 1);
        assert_eq!(d.decisions[0].decision, "Storage layer");
        assert_eq!(d.decisions[0].chosen, "PostgreSQL");
        assert_eq!(d.decisions[0].alternatives, "SQLite, DynamoDB");
        assert_eq!(d.decisions[0].rationale, "Existing infra");
        assert!(!d.decisions[0].inferred);
    }

    #[test]
    fn lld_detects_inferred_marker_in_any_cell() {
        let content = "\
## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| Session store | Redis | DB table | Latency [inferred] |
| Cache key shape | Hash | Plain | Avoid collisions |
";
        let d = lld(content);
        assert_eq!(d.decisions.len(), 2);
        assert!(d.decisions[0].inferred);
        assert!(!d.decisions[1].inferred);
    }

    #[test]
    fn lld_stops_capturing_at_next_h2() {
        let content = "\
## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| A | B | C | D |

## Open Questions

| not | a | decision | row |
| --- | --- | --- | --- |
| should | not | be | captured |
";
        let d = lld(content);
        assert_eq!(d.decisions.len(), 1);
        assert_eq!(d.decisions[0].decision, "A");
    }

    #[test]
    fn lld_accepts_decisions_and_alternatives_alt_spelling() {
        // Some authors write "Decisions and Alternatives" (no ampersand).
        let content = "\
## Decisions and Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| X | Y | Z | because |
";
        let d = lld(content);
        assert_eq!(d.decisions.len(), 1);
    }

    #[test]
    fn lld_pads_short_rows_to_column_count() {
        // pulldown-cmark pads rows that have fewer cells than the header
        // to match the header column count (GFM spec §4.10). A two-cell
        // row in a four-column table becomes ["short", "row", "", ""].
        let content = "\
## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| short | row |
| Valid | Row | Yes | Indeed |
";
        let d = lld(content);
        assert_eq!(d.decisions.len(), 2);
        assert_eq!(d.decisions[0].decision, "short");
        assert!(d.decisions[0].alternatives.is_empty());
        assert_eq!(d.decisions[1].decision, "Valid");
    }

    #[test]
    fn update_spec_status_replaces_marker() {
        let content =
            "- [ ] **AUTH-001**: users shall log in.\n- [x] **AUTH-002**: sessions expire.\n";
        let id = SpecId::parse("AUTH-001").unwrap();
        let updated = update_spec_status_in_text(content, &id, SpecStatus::Implemented).unwrap();
        assert!(updated.contains("- [x] **AUTH-001**: users shall log in."));
        assert!(updated.contains("- [x] **AUTH-002**: sessions expire."));
        assert!(updated.ends_with('\n'));
    }

    #[test]
    fn update_spec_status_returns_none_when_id_not_found() {
        let content = "- [ ] **AUTH-001**: text.\n";
        let id = SpecId::parse("AUTH-999").unwrap();
        assert!(update_spec_status_in_text(content, &id, SpecStatus::Open).is_none());
    }

    #[test]
    fn parse_spec_file_extracts_yaml_frontmatter_prefix() {
        let content = "---\nprefix: AUTH\n---\n- [ ] **AUTH-001**: text.\n";
        let path = std::path::Path::new("auth-specs.md");
        let sf = parse_spec_file(content, path).unwrap();
        assert_eq!(sf.prefix.as_deref(), Some("AUTH"));
        assert_eq!(sf.specs.len(), 1);
    }

    #[test]
    fn parse_spec_file_no_frontmatter_yields_none_prefix() {
        let content = "- [ ] **AUTH-001**: text.\n";
        let path = std::path::Path::new("auth-specs.md");
        let sf = parse_spec_file(content, path).unwrap();
        assert!(sf.prefix.is_none());
    }

    #[test]
    fn parse_spec_file_frontmatter_without_prefix_key_yields_none() {
        let content = "---\ntitle: Auth\n---\n- [ ] **AUTH-001**: text.\n";
        let path = std::path::Path::new("auth-specs.md");
        let sf = parse_spec_file(content, path).unwrap();
        assert!(sf.prefix.is_none());
    }

    #[test]
    fn update_spec_status_preserves_trailing_newline() {
        let with_newline = "- [ ] **AUTH-001**: text.\n";
        let without_newline = "- [ ] **AUTH-001**: text.";
        let id = SpecId::parse("AUTH-001").unwrap();
        let updated_with =
            update_spec_status_in_text(with_newline, &id, SpecStatus::Deferred).unwrap();
        let updated_without =
            update_spec_status_in_text(without_newline, &id, SpecStatus::Deferred).unwrap();
        assert!(updated_with.ends_with('\n'));
        assert!(!updated_without.ends_with('\n'));
    }

    // ── Decision doc parser ────────────────────────────────────────────

    #[test]
    fn parse_decision_doc_extracts_h1_title() {
        let content = "# Session Storage Strategy\n\nWe chose Redis because…\n";
        let doc = parse_decision_doc(
            content,
            Path::new("docs/decisions/session.md"),
            DecisionScope::Project,
        );
        assert_eq!(doc.title, "Session Storage Strategy");
        assert_eq!(doc.scope, DecisionScope::Project);
    }

    #[test]
    fn parse_decision_doc_trims_whitespace_in_title() {
        let content = "#   Spaced Title  \n\nProse.\n";
        let doc = parse_decision_doc(content, Path::new("foo.md"), DecisionScope::Project);
        assert_eq!(doc.title, "Spaced Title");
    }

    #[test]
    fn parse_decision_doc_returns_empty_title_when_no_h1() {
        let content = "## Not an H1\n\nSome prose.\n";
        let doc = parse_decision_doc(content, Path::new("foo.md"), DecisionScope::Project);
        assert!(
            doc.title.is_empty(),
            "expected empty title, got {:?}",
            doc.title
        );
    }

    #[test]
    fn parse_decision_doc_node_scope_is_preserved() {
        let content = "# Token Format\n";
        let doc = parse_decision_doc(
            content,
            Path::new("docs/intent/auth/decisions/token.md"),
            DecisionScope::Node {
                segment: "auth".to_owned(),
            },
        );
        assert_eq!(
            doc.scope,
            DecisionScope::Node {
                segment: "auth".to_owned()
            }
        );
    }
}
