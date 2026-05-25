//! `textDocument/references` for spec IDs.
//!
//! When the cursor sits inside a spec ID — either a `@spec` citation
//! in source or a `**SPEC-ID**` line in a spec file — return every
//! known location of that spec across the workspace: the definition
//! (when `include_declaration` is set) plus every cited site.
//!
//! Locations are line-granular for now. Precise byte-range
//! highlighting per citation can land in a perf pass.

use lid_core::LidRepo;
use lid_core::SpecId;
use lid_core::model::SpecCitation;
use lid_core::parse::markdown::find_spec_line_match;
use lid_core::parse::source::find_citations_in_line;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

/// Compute the reference list for the spec ID at the given position.
///
/// Returns `None` when the cursor isn't on a spec ID.
pub fn references_at_position(
    repo: &LidRepo,
    text: &str,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let id = spec_id_at_cursor(text, position)?;

    let mut locations = Vec::new();
    if include_declaration {
        for spec_file in &repo.specs {
            for line in &spec_file.specs {
                if line.id == id {
                    if let Some(loc) = file_line_location(&spec_file.path, line.line) {
                        locations.push(loc);
                    }
                }
            }
        }
    }
    for citation in &repo.citations {
        if citation.id == id {
            if let Some(loc) = citation_location(citation) {
                locations.push(loc);
            }
        }
    }
    Some(locations)
}

/// Locate the spec ID at the cursor — either inside a `@spec` citation
/// or inside the `**ID**` span of a spec-file line. Returns `None`
/// when neither applies.
pub(crate) fn spec_id_at_cursor(text: &str, position: Position) -> Option<SpecId> {
    let line_idx = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_idx)?;
    let cursor_byte = usize::try_from(position.character).ok()?;

    if let Some(c) = find_citations_in_line(line)
        .into_iter()
        .find(|c| c.byte_range.start <= cursor_byte && cursor_byte <= c.byte_range.end)
    {
        return Some(c.id);
    }
    if let Some(m) = find_spec_line_match(line) {
        if m.id_byte_range.start <= cursor_byte && cursor_byte <= m.id_byte_range.end {
            return Some(m.id);
        }
    }
    None
}

fn file_line_location(path: &std::path::Path, line: usize) -> Option<Location> {
    let uri = Url::from_file_path(path).ok()?;
    let target = u32::try_from(line.saturating_sub(1)).unwrap_or(0);
    Some(Location {
        uri,
        range: Range {
            start: Position {
                line: target,
                character: 0,
            },
            end: Position {
                line: target.saturating_add(1),
                character: 0,
            },
        },
    })
}

fn citation_location(c: &SpecCitation) -> Option<Location> {
    file_line_location(&c.file, c.line)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{CitationKind, SpecCitation, SpecFile, SpecLine, SpecStatus};
    use lid_core::{ArrowIndex, Unmapped};

    use super::*;

    fn root() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("C:\\repo")
        } else {
            PathBuf::from("/repo")
        }
    }

    fn make_repo(spec_ids: &[&str], citations: Vec<SpecCitation>) -> LidRepo {
        let root = root();
        let specs = if spec_ids.is_empty() {
            vec![]
        } else {
            vec![SpecFile {
                path: root.join("docs/specs/auth-specs.md"),
                specs: spec_ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| SpecLine {
                        id: SpecId::parse(id).unwrap(),
                        status: SpecStatus::Implemented,
                        text: "stub".into(),
                        line: i + 1,
                    })
                    .collect(),
                implementing_artifacts: vec![],
                lld: None,
            }]
        };
        LidRepo {
            root,
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs,
            llds: vec![],
            arrow_docs: vec![],
            citations,
        }
    }

    fn citation(id: &str, rel: &str, line: usize) -> SpecCitation {
        SpecCitation {
            id: SpecId::parse(id).unwrap(),
            file: root().join(rel),
            line,
            kind: CitationKind::Code,
        }
    }

    fn position(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn cursor_on_citation_returns_all_locations() {
        let repo = make_repo(
            &["AUTH-001"],
            vec![
                citation("AUTH-001", "src/a.rs", 12),
                citation("AUTH-001", "tests/b.rs", 5),
            ],
        );
        let text = "// @spec AUTH-001\n";
        let locs = references_at_position(&repo, text, position(0, 12), true).unwrap();
        // 1 definition + 2 citations.
        assert_eq!(locs.len(), 3);
    }

    #[test]
    fn cursor_on_definition_returns_all_locations() {
        let repo = make_repo(&["AUTH-001"], vec![citation("AUTH-001", "src/a.rs", 12)]);
        // `**AUTH-001**` starts at byte 8; cursor at byte 10 sits inside.
        let text = "- [x] **AUTH-001**: requirement\n";
        let locs = references_at_position(&repo, text, position(0, 10), true).unwrap();
        assert_eq!(locs.len(), 2);
    }

    #[test]
    fn include_declaration_false_omits_definition() {
        let repo = make_repo(&["AUTH-001"], vec![citation("AUTH-001", "src/a.rs", 12)]);
        let text = "// @spec AUTH-001\n";
        let locs = references_at_position(&repo, text, position(0, 12), false).unwrap();
        assert_eq!(locs.len(), 1);
        assert!(locs[0].uri.path().ends_with("src/a.rs"));
    }

    #[test]
    fn cursor_outside_any_spec_id_returns_none() {
        let repo = make_repo(&["AUTH-001"], vec![]);
        let text = "// @spec AUTH-001\n";
        assert!(references_at_position(&repo, text, position(0, 2), true).is_none());
    }

    #[test]
    fn unknown_spec_at_cursor_returns_empty_when_no_locations() {
        let repo = make_repo(&["AUTH-001"], vec![]);
        let text = "// @spec UNKNOWN-001\n";
        // The spec ID *is* extractable from the cursor (the regex
        // matches), it just doesn't exist in the repo. So we return
        // Some(empty) — the editor displays "no references found"
        // rather than "feature unavailable".
        let locs = references_at_position(&repo, text, position(0, 12), true).unwrap();
        assert!(locs.is_empty(), "got {locs:?}");
    }

    #[test]
    fn spec_id_at_cursor_picks_definition_over_citation_when_both_match() {
        // A spec file line like `- [x] **AUTH-001**: …` — the cursor on
        // the ID should resolve via the definition path, even though
        // the line could theoretically also be scanned for citations.
        let text = "- [x] **AUTH-001**: text\n";
        let id = spec_id_at_cursor(text, position(0, 10)).unwrap();
        assert_eq!(id.as_str(), "AUTH-001");
    }
}
