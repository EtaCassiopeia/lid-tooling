//! `textDocument/definition` for `@spec SPEC-ID` citations.
//!
//! When the cursor sits inside an `@spec` reference, return a
//! [`Location`] pointing at the line in the spec file that defines
//! that spec. The range covers the entire line (column 0 to end of
//! line) so editors that highlight the destination select the whole
//! requirement rather than just one character.

use lid_core::LidRepo;
use lid_core::parse::source::find_citations_in_line;
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};

/// Compute the go-to-definition response for the given cursor position.
///
/// Returns `None` when:
/// * the cursor is past EOF or outside any `@spec` citation,
/// * the cited spec ID isn't defined in the repository, or
/// * the spec file path can't be encoded as a `file://` URL (only
///   happens for non-absolute paths, which shouldn't occur for paths
///   produced by `LidRepo::discover`).
pub fn definition_at_position(
    repo: &LidRepo,
    text: &str,
    position: Position,
) -> Option<GotoDefinitionResponse> {
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

    let uri = Url::from_file_path(&spec_file.path).ok()?;
    // `SpecLine.line` is 1-based to match editor row numbers; LSP
    // positions are 0-based, so subtract one (saturating in the
    // pathological case where the spec line was recorded as 0).
    let target_line = u32::try_from(spec_line.line.saturating_sub(1)).unwrap_or(0);

    let location = Location {
        uri,
        range: Range {
            start: Position {
                line: target_line,
                character: 0,
            },
            end: Position {
                line: target_line.saturating_add(1),
                character: 0,
            },
        },
    };
    Some(GotoDefinitionResponse::Scalar(location))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{SpecFile, SpecLine, SpecStatus};
    use lid_core::{ArrowIndex, SpecId, Unmapped};

    use super::*;

    fn repo_with_spec(spec_line_number: usize) -> LidRepo {
        // Use an absolute path so Url::from_file_path succeeds.
        let root = if cfg!(windows) {
            PathBuf::from("C:\\repo")
        } else {
            PathBuf::from("/repo")
        };
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
                    text: "text".into(),
                    line: spec_line_number,
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

    fn unwrap_scalar(resp: GotoDefinitionResponse) -> Location {
        match resp {
            GotoDefinitionResponse::Scalar(loc) => loc,
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    #[test]
    fn cursor_inside_citation_returns_spec_location() {
        let repo = repo_with_spec(5);
        let text = "// @spec AUTH-001\n";
        let resp = definition_at_position(&repo, text, position(0, 12)).unwrap();
        let loc = unwrap_scalar(resp);
        assert!(
            loc.uri.path().ends_with("docs/specs/auth-specs.md"),
            "got: {}",
            loc.uri
        );
        // SpecLine.line = 5 (1-based) → LSP line 4 (0-based).
        assert_eq!(loc.range.start.line, 4);
        assert_eq!(loc.range.end.line, 5);
    }

    #[test]
    fn cursor_outside_citation_returns_none() {
        let repo = repo_with_spec(5);
        let text = "// @spec AUTH-001\n";
        assert!(definition_at_position(&repo, text, position(0, 2)).is_none());
    }

    #[test]
    fn cursor_on_unknown_spec_returns_none() {
        let repo = repo_with_spec(5);
        let text = "// @spec UNKNOWN-001\n";
        assert!(definition_at_position(&repo, text, position(0, 12)).is_none());
    }

    #[test]
    fn past_eof_returns_none() {
        let repo = repo_with_spec(5);
        let text = "// @spec AUTH-001\n";
        assert!(definition_at_position(&repo, text, position(99, 0)).is_none());
    }

    #[test]
    fn line_one_translates_to_lsp_zero() {
        let repo = repo_with_spec(1);
        let text = "// @spec AUTH-001\n";
        let resp = definition_at_position(&repo, text, position(0, 12)).unwrap();
        let loc = unwrap_scalar(resp);
        assert_eq!(loc.range.start.line, 0);
        assert_eq!(loc.range.end.line, 1);
    }
}
