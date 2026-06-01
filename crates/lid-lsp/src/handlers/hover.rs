//! `textDocument/hover` for LID artifacts.
//!
//! Two hover sites:
//!
//! * **Citation hover.** Cursor on `@spec SPEC-ID` in source — show
//!   the spec text, status, and where it's defined.
//! * **Definition hover.** Cursor on `**SPEC-ID**` in a spec file
//!   line — show the spec text and every place the source scanner
//!   has cited it.
//!
//! Both share `hover_at_position`, which tries each variant in turn.

use std::fmt::Write as _;
use std::path::Path;

use lid_core::model::SpecStatus;
use lid_core::parse::markdown::{SpecLineMatch, find_spec_line_match};
use lid_core::parse::source::find_citations_in_line;
use lid_core::{LidRepo, SpecId};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

/// Compute the hover response for the cursor position in `text`.
///
/// Tries citation hover first (the common case when editing source);
/// falls back to definition hover (when editing a spec file).
pub fn hover_at_position(repo: &LidRepo, text: &str, position: Position) -> Option<Hover> {
    citation_hover(repo, text, position).or_else(|| definition_hover(repo, text, position))
}

/// Hover when the cursor sits on `@spec SPEC-ID` in source.
fn citation_hover(repo: &LidRepo, text: &str, position: Position) -> Option<Hover> {
    let line = line_at(text, position.line)?;
    let cursor_byte = usize::try_from(position.character).ok()?;

    let citation = find_citations_in_line(line)
        .into_iter()
        .find(|c| c.byte_range.start <= cursor_byte && cursor_byte <= c.byte_range.end)?;

    let (spec_file, spec_line) = repo
        .specs
        .iter()
        .find_map(|f| f.specs.iter().find(|s| s.id == citation.id).map(|s| (f, s)))?;

    let body = render_citation_markdown(repo, &citation.id, spec_line, &spec_file.path);
    Some(make_hover(body, position.line, citation.byte_range))
}

/// Hover when the cursor sits on `**SPEC-ID**` in a spec file line.
fn definition_hover(repo: &LidRepo, text: &str, position: Position) -> Option<Hover> {
    let line = line_at(text, position.line)?;
    let cursor_byte = usize::try_from(position.character).ok()?;

    let m = find_spec_line_match(line)?;
    if cursor_byte < m.id_byte_range.start || cursor_byte > m.id_byte_range.end {
        return None;
    }

    let citations: Vec<_> = repo.citations.iter().filter(|c| c.id == m.id).collect();

    let body = render_definition_markdown(repo, &m, &citations);
    Some(make_hover(body, position.line, m.id_byte_range))
}

fn line_at(text: &str, line: u32) -> Option<&str> {
    let idx = usize::try_from(line).ok()?;
    text.lines().nth(idx)
}

fn make_hover(body: String, line: u32, byte_range: std::ops::Range<usize>) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: body,
        }),
        range: Some(Range {
            start: Position {
                line,
                character: u32::try_from(byte_range.start).unwrap_or(u32::MAX),
            },
            end: Position {
                line,
                character: u32::try_from(byte_range.end).unwrap_or(u32::MAX),
            },
        }),
    }
}

fn render_citation_markdown(
    repo: &LidRepo,
    id: &SpecId,
    spec_line: &lid_core::model::SpecLine,
    spec_file_path: &Path,
) -> String {
    let rel = spec_file_path
        .strip_prefix(&repo.root)
        .unwrap_or(spec_file_path);
    format!(
        "**`{id}`** — {status}\n\n> {text}\n\nDefined at `{rel}:{line}`",
        status = status_label(spec_line.status),
        text = spec_line.text,
        rel = rel.display(),
        line = spec_line.line,
    )
}

fn render_definition_markdown(
    repo: &LidRepo,
    m: &SpecLineMatch,
    citations: &[&lid_core::model::SpecCitation],
) -> String {
    let mut out = format!(
        "**`{id}`** — {status}\n\n> {text}",
        id = m.id,
        status = status_label(m.status),
        text = m.text,
    );
    if citations.is_empty() {
        out.push_str("\n\nNo `@spec` citations found in source.");
    } else {
        let mut count_line = format!("\n\nCited in {} place", citations.len());
        if citations.len() != 1 {
            count_line.push('s');
        }
        count_line.push(':');
        out.push_str(&count_line);
        for c in citations {
            let rel = c.file.strip_prefix(&repo.root).unwrap_or(&c.file);
            let _ = write!(out, "\n- `{}:{}`", rel.display(), c.line);
        }
    }
    out
}

fn status_label(s: SpecStatus) -> &'static str {
    match s {
        SpecStatus::Implemented => "`[x]` implemented",
        SpecStatus::Open => "`[ ]` open",
        SpecStatus::Deferred => "`[D]` deferred",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{CitationKind, SpecCitation, SpecFile, SpecLine};
    use lid_core::{ArrowIndex, Unmapped};

    use super::*;

    fn make_repo_with_citations(
        spec_text: &str,
        spec_status: SpecStatus,
        citation_lines: &[(&str, usize, CitationKind)],
    ) -> LidRepo {
        let root = PathBuf::from("/repo");
        let citations = citation_lines
            .iter()
            .map(|(path, line, kind)| SpecCitation {
                id: SpecId::parse("AUTH-001").unwrap(),
                file: root.join(path),
                line: *line,
                kind: *kind,
            })
            .collect();
        LidRepo {
            root: root.clone(),
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs: vec![SpecFile {
                path: root.join("docs/specs/auth-specs.md"),
                specs: vec![SpecLine {
                    id: SpecId::parse("AUTH-001").unwrap(),
                    status: spec_status,
                    text: spec_text.to_owned(),
                    line: 5,
                }],
                implementing_artifacts: vec![],
                lld: None,
                prefix: None,
            }],
            llds: vec![],
            arrow_docs: vec![],
            citations,
            decision_docs: vec![],
        }
    }

    fn position(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    // ── Citation hover (M3.24 regression coverage) ────────────────

    #[test]
    fn citation_hover_inside_spec_id_returns_spec_text() {
        let repo = make_repo_with_citations("logs in", SpecStatus::Implemented, &[]);
        let text = "// @spec AUTH-001\n";
        let hover = hover_at_position(&repo, text, position(0, 12)).unwrap();
        match hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("AUTH-001"), "got: {}", m.value);
                assert!(m.value.contains("logs in"), "got: {}", m.value);
                assert!(m.value.contains("Defined at"), "got: {}", m.value);
            }
            other => panic!("expected markup, got {other:?}"),
        }
    }

    #[test]
    fn citation_hover_outside_spec_id_returns_none() {
        let repo = make_repo_with_citations("text", SpecStatus::Implemented, &[]);
        let text = "// @spec AUTH-001\n";
        assert!(hover_at_position(&repo, text, position(0, 2)).is_none());
    }

    #[test]
    fn citation_hover_at_left_edge_of_spec_id_resolves() {
        let repo = make_repo_with_citations("text", SpecStatus::Implemented, &[]);
        let text = "// @spec AUTH-001\n";
        assert!(hover_at_position(&repo, text, position(0, 9)).is_some());
    }

    #[test]
    fn citation_hover_at_right_edge_of_spec_id_resolves() {
        let repo = make_repo_with_citations("text", SpecStatus::Implemented, &[]);
        let text = "// @spec AUTH-001\n";
        assert!(hover_at_position(&repo, text, position(0, 17)).is_some());
    }

    #[test]
    fn citation_hover_for_unknown_spec_returns_none() {
        let repo = make_repo_with_citations("text", SpecStatus::Implemented, &[]);
        let text = "// @spec UNKNOWN-001\n";
        assert!(hover_at_position(&repo, text, position(0, 12)).is_none());
    }

    #[test]
    fn citation_hover_on_line_without_at_spec_returns_none() {
        let repo = make_repo_with_citations("text", SpecStatus::Implemented, &[]);
        let text = "const SPECS = ['AUTH-001'];\n";
        assert!(hover_at_position(&repo, text, position(0, 18)).is_none());
    }

    #[test]
    fn hover_past_eof_returns_none() {
        let repo = make_repo_with_citations("text", SpecStatus::Implemented, &[]);
        let text = "// @spec AUTH-001\n";
        assert!(hover_at_position(&repo, text, position(99, 0)).is_none());
    }

    #[test]
    fn citation_hover_picks_correct_id_when_line_has_multiple() {
        let repo = make_repo_with_citations("text", SpecStatus::Implemented, &[]);
        let text = "// @spec AUTH-001, AUTH-002\n";
        // Column 22 is inside AUTH-002. Only AUTH-001 is in the repo,
        // so the lookup should not fall through.
        assert!(hover_at_position(&repo, text, position(0, 22)).is_none());
    }

    // ── Definition hover (new in M3.25) ──────────────────────────

    #[test]
    fn definition_hover_shows_citation_list() {
        let repo = make_repo_with_citations(
            "When the user logs in, …",
            SpecStatus::Implemented,
            &[
                ("src/auth/login.ts", 42, CitationKind::Code),
                ("tests/auth.test.ts", 5, CitationKind::Test),
            ],
        );
        let text = "- [x] **AUTH-001**: When the user logs in, …\n";
        // Cursor at column 10 — inside the bold AUTH-001 span
        // (bytes 8..=15 in the line).
        let hover = hover_at_position(&repo, text, position(0, 10)).unwrap();
        match hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("AUTH-001"), "got: {}", m.value);
                assert!(m.value.contains("`[x]` implemented"), "got: {}", m.value);
                assert!(m.value.contains("Cited in 2 places"), "got: {}", m.value);
                assert!(m.value.contains("src/auth/login.ts:42"), "got: {}", m.value);
                assert!(m.value.contains("tests/auth.test.ts:5"), "got: {}", m.value);
            }
            other => panic!("expected markup, got {other:?}"),
        }
    }

    #[test]
    fn definition_hover_with_no_citations_says_so() {
        let repo = make_repo_with_citations("text", SpecStatus::Open, &[]);
        let text = "- [ ] **AUTH-001**: text\n";
        let hover = hover_at_position(&repo, text, position(0, 10)).unwrap();
        match hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("No `@spec` citations"), "got: {}", m.value);
                assert!(m.value.contains("`[ ]` open"), "got: {}", m.value);
            }
            other => panic!("expected markup, got {other:?}"),
        }
    }

    #[test]
    fn definition_hover_singular_when_one_citation() {
        let repo = make_repo_with_citations(
            "text",
            SpecStatus::Implemented,
            &[("src/x.rs", 1, CitationKind::Code)],
        );
        let text = "- [x] **AUTH-001**: text\n";
        let hover = hover_at_position(&repo, text, position(0, 10)).unwrap();
        match hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("Cited in 1 place:"), "got: {}", m.value);
                assert!(!m.value.contains("1 places"), "expected singular");
            }
            other => panic!("expected markup, got {other:?}"),
        }
    }

    #[test]
    fn definition_hover_outside_id_span_returns_none() {
        let repo = make_repo_with_citations("text", SpecStatus::Open, &[]);
        let text = "- [x] **AUTH-001**: text\n";
        // Cursor on the requirement text, well past the bold ID span.
        assert!(hover_at_position(&repo, text, position(0, 22)).is_none());
    }

    #[test]
    fn definition_hover_returns_range_covering_id() {
        let repo = make_repo_with_citations("text", SpecStatus::Open, &[]);
        let text = "- [x] **AUTH-001**: text\n";
        let hover = hover_at_position(&repo, text, position(0, 10)).unwrap();
        let range = hover.range.unwrap();
        // The captured ID span is bytes 8..=15 in `- [x] **AUTH-001**: …`.
        assert_eq!(range.start.character, 8);
        assert_eq!(range.end.character, 16);
    }

    #[test]
    fn definition_hover_returns_none_on_non_spec_line() {
        let repo = make_repo_with_citations("text", SpecStatus::Open, &[]);
        let text = "## Section header\n";
        assert!(hover_at_position(&repo, text, position(0, 5)).is_none());
    }
}
