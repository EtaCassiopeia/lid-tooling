//! `textDocument/hover` for `@spec SPEC-ID` citations in source.
//!
//! When the cursor sits on (or just past) a `@spec`-referenced spec ID
//! in a source file, the hover popup shows the spec text, its current
//! status (`[x]` / `[ ]` / `[D]`), and a pointer back to the spec
//! definition. Lines without a `@spec` keyword, cursor positions
//! outside any spec ID, and unknown spec IDs all return `None`.

use lid_core::model::SpecStatus;
use lid_core::parse::source::find_citations_in_line;
use lid_core::{LidRepo, SpecId};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

/// Compute the hover response for the given cursor position in `text`.
///
/// Returns `None` when:
/// * the cursor's line is past the document end,
/// * no `@spec` citation surrounds the cursor's column, or
/// * the cited spec ID isn't defined in the repository.
pub fn hover_at_position(repo: &LidRepo, text: &str, position: Position) -> Option<Hover> {
    let line_idx = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_idx)?;
    let cursor_byte = usize::try_from(position.character).ok()?;

    let citation = find_citations_in_line(line)
        .into_iter()
        .find(|c| c.byte_range.start <= cursor_byte && cursor_byte <= c.byte_range.end)?;

    let (spec_file, spec_line) = repo
        .specs
        .iter()
        .find_map(|f| f.specs.iter().find(|s| s.id == citation.id).map(|s| (f, s)))?;

    let body = render_markdown(repo, &citation.id, spec_line, &spec_file.path);

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: body,
        }),
        range: Some(Range {
            start: Position {
                line: position.line,
                character: u32::try_from(citation.byte_range.start).unwrap_or(u32::MAX),
            },
            end: Position {
                line: position.line,
                character: u32::try_from(citation.byte_range.end).unwrap_or(u32::MAX),
            },
        }),
    })
}

fn render_markdown(
    repo: &LidRepo,
    id: &SpecId,
    spec_line: &lid_core::model::SpecLine,
    spec_file_path: &std::path::Path,
) -> String {
    let rel = spec_file_path
        .strip_prefix(&repo.root)
        .unwrap_or(spec_file_path);
    let status = match spec_line.status {
        SpecStatus::Implemented => "`[x]` implemented",
        SpecStatus::Open => "`[ ]` open",
        SpecStatus::Deferred => "`[D]` deferred",
    };
    format!(
        "**`{id}`** — {status}\n\n> {text}\n\nDefined at `{rel}:{line}`",
        text = spec_line.text,
        rel = rel.display(),
        line = spec_line.line,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{SpecFile, SpecLine, SpecStatus};
    use lid_core::{ArrowIndex, Unmapped};

    use super::*;

    fn make_repo(spec_text: &str) -> LidRepo {
        let root = PathBuf::from("/repo");
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
                    status: SpecStatus::Implemented,
                    text: spec_text.to_owned(),
                    line: 5,
                }],
                implementing_artifacts: vec![],
                lld: None,
            }],
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
        }
    }

    fn position(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn hover_inside_spec_id_returns_spec_text() {
        let repo = make_repo("When the user logs in, the system SHALL ...");
        let text = "// @spec AUTH-001\nfn login() {}\n";
        // Cursor at column 12 (middle of "AUTH-001" on line 0).
        let hover = hover_at_position(&repo, text, position(0, 12)).unwrap();
        match hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("AUTH-001"), "got: {}", m.value);
                assert!(m.value.contains("logs in"), "got: {}", m.value);
                assert!(m.value.contains("`[x]` implemented"), "got: {}", m.value);
                assert!(
                    m.value.contains("docs/specs/auth-specs.md:5"),
                    "got: {}",
                    m.value
                );
            }
            other => panic!("expected Markup contents, got {other:?}"),
        }
        let range = hover.range.unwrap();
        // The "AUTH-001" span in `// @spec AUTH-001` is bytes 9..=17.
        assert_eq!(range.start.character, 9);
        assert_eq!(range.end.character, 17);
    }

    #[test]
    fn hover_at_left_edge_of_spec_id_still_resolves() {
        let repo = make_repo("text");
        let text = "// @spec AUTH-001\n";
        let hover = hover_at_position(&repo, text, position(0, 9));
        assert!(hover.is_some(), "cursor at start of ID should resolve");
    }

    #[test]
    fn hover_at_right_edge_of_spec_id_still_resolves() {
        let repo = make_repo("text");
        let text = "// @spec AUTH-001\n";
        let hover = hover_at_position(&repo, text, position(0, 17));
        assert!(hover.is_some(), "cursor at end of ID should resolve");
    }

    #[test]
    fn hover_outside_spec_id_returns_none() {
        let repo = make_repo("text");
        let text = "// @spec AUTH-001\n";
        // Cursor at column 2 — in the comment marker, before @spec.
        assert!(hover_at_position(&repo, text, position(0, 2)).is_none());
    }

    #[test]
    fn hover_on_line_without_at_spec_returns_none() {
        let repo = make_repo("text");
        let text = "const SPECS = ['AUTH-001'];\n";
        // AUTH-001 appears but no @spec keyword — no hover.
        assert!(hover_at_position(&repo, text, position(0, 18)).is_none());
    }

    #[test]
    fn hover_for_unknown_spec_returns_none() {
        let repo = make_repo("text");
        let text = "// @spec UNKNOWN-001\nfn x() {}\n";
        assert!(hover_at_position(&repo, text, position(0, 12)).is_none());
    }

    #[test]
    fn hover_past_eof_returns_none() {
        let repo = make_repo("text");
        let text = "// @spec AUTH-001\n";
        // Line 99 doesn't exist.
        assert!(hover_at_position(&repo, text, position(99, 0)).is_none());
    }

    #[test]
    fn hover_picks_correct_id_when_line_has_multiple() {
        let repo = make_repo("text");
        let text = "// @spec AUTH-001, AUTH-002\n";
        // Column 22 is inside "AUTH-002" (bytes 19..=26).
        let hover = hover_at_position(&repo, text, position(0, 22));
        // AUTH-002 isn't in the repo, so we get None; this also proves
        // the lookup pinned the right citation (didn't fall through to
        // AUTH-001 just because it exists).
        assert!(hover.is_none(), "should not fall through to AUTH-001");
    }
}
