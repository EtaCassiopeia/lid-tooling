//! `textDocument/prepareRename` and `textDocument/rename` for spec IDs.
//!
//! Renaming a spec ID is a workspace-wide operation: the
//! `**SPEC-ID**` token in the spec file *and* every `@spec` citation
//! in source/tests update together so traceability stays intact.
//!
//! The handler:
//!
//! 1. resolves the spec ID at the cursor (citation or definition),
//! 2. validates the new name (`SpecId::parse`) and rejects collisions
//!    with an existing spec,
//! 3. reads every affected file (preferring the editor buffer in
//!    `DocStore`, falling back to disk) to compute precise byte
//!    ranges, and
//! 4. returns a `WorkspaceEdit` keyed by URI for the editor to apply.
//!
//! Reading from `DocStore` first matters: VS Code applies the
//! workspace edit to the editor's *buffer*, not the disk file, so
//! offsets derived from stale disk content would corrupt buffers
//! with unsaved changes.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use lid_core::parse::markdown::find_spec_line_match;
use lid_core::parse::source::find_citations_in_line;
use lid_core::{DocStore, LidRepo, SpecId, model::SpecCitation};
use thiserror::Error;
use tower_lsp::lsp_types::{Position, PrepareRenameResponse, Range, TextEdit, Url, WorkspaceEdit};

use super::references::spec_id_at_cursor;

/// Errors a rename can produce that should surface to the user.
#[derive(Debug, Error)]
pub enum RenameError {
    #[error("cursor is not on a spec ID")]
    NotOnSpec,
    #[error("`{value}` is not a valid spec ID")]
    InvalidNewName { value: String },
    #[error("spec ID `{id}` is already in use")]
    Collision { id: String },
}

/// Validate the cursor sits on a spec ID and return the range the
/// editor should let the user type into.
pub fn prepare_rename_at_position(text: &str, position: Position) -> Option<PrepareRenameResponse> {
    let line_idx = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_idx)?;
    let cursor_byte = usize::try_from(position.character).ok()?;

    if let Some(c) = find_citations_in_line(line)
        .into_iter()
        .find(|c| c.byte_range.start <= cursor_byte && cursor_byte <= c.byte_range.end)
    {
        return Some(PrepareRenameResponse::Range(byte_range_to_lsp_range(
            position.line,
            &c.byte_range,
        )));
    }
    if let Some(m) = find_spec_line_match(line) {
        if m.id_byte_range.start <= cursor_byte && cursor_byte <= m.id_byte_range.end {
            return Some(PrepareRenameResponse::Range(byte_range_to_lsp_range(
                position.line,
                &m.id_byte_range,
            )));
        }
    }
    None
}

/// Compute the workspace edit for renaming the spec ID at the cursor.
///
/// # Errors
/// * [`RenameError::NotOnSpec`] when the cursor isn't on a spec ID.
/// * [`RenameError::InvalidNewName`] when `new_name` fails
///   `SpecId::parse`.
/// * [`RenameError::Collision`] when `new_name` already names another
///   spec in the repository.
pub fn rename_at_position(
    repo: &LidRepo,
    store: &DocStore,
    text: &str,
    position: Position,
    new_name: &str,
) -> Result<WorkspaceEdit, RenameError> {
    let old_id = spec_id_at_cursor(text, position).ok_or(RenameError::NotOnSpec)?;
    let new_id = SpecId::parse(new_name).map_err(|_| RenameError::InvalidNewName {
        value: new_name.to_owned(),
    })?;

    if new_id == old_id {
        return Ok(WorkspaceEdit::default());
    }
    if repo
        .specs
        .iter()
        .flat_map(|f| f.specs.iter())
        .any(|s| s.id == new_id)
    {
        return Err(RenameError::Collision {
            id: new_id.to_string(),
        });
    }

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

    // Edit the spec definition site(s). For each candidate spec
    // file, fetch the current content — buffer first via DocStore,
    // then disk — and scan every line for spec definitions matching
    // `old_id`. We deliberately *don't* trust `SpecLine.line` from
    // `repo.specs`: that number was captured at discovery time
    // against the on-disk content, and the buffer may have shifted
    // since. Scanning fresh content is the only way the edit's
    // line/byte ranges match what the editor will apply against.
    for spec_file in &repo.specs {
        let needed = spec_file.specs.iter().any(|s| s.id == old_id);
        if !needed {
            continue;
        }
        let Some((uri, content)) = read_path(store, &spec_file.path) else {
            continue;
        };
        for (line_idx, line) in content.lines().enumerate() {
            let Some(m) = find_spec_line_match(line) else {
                continue;
            };
            if m.id != old_id {
                continue;
            }
            changes.entry(uri.clone()).or_default().push(TextEdit {
                range: byte_range_to_lsp_range(
                    u32::try_from(line_idx).unwrap_or(u32::MAX),
                    &m.id_byte_range,
                ),
                new_text: new_name.to_owned(),
            });
        }
    }

    // Edit every citation site. Group by file so we read each file
    // once, then scan its lines for matches against `old_id`.
    let citation_files: BTreeSet<&Path> = repo
        .citations
        .iter()
        .filter(|c: &&SpecCitation| c.id == old_id)
        .map(|c| c.file.as_path())
        .collect();
    // BTreeSet collects in path order so iteration over files is deterministic.
    for path in citation_files {
        let Some((uri, content)) = read_path(store, path) else {
            continue;
        };
        let mut edits = Vec::new();
        for (line_idx, line) in content.lines().enumerate() {
            for lc in find_citations_in_line(line) {
                if lc.id == old_id {
                    edits.push(TextEdit {
                        range: byte_range_to_lsp_range(
                            u32::try_from(line_idx).unwrap_or(u32::MAX),
                            &lc.byte_range,
                        ),
                        new_text: new_name.to_owned(),
                    });
                }
            }
        }
        if !edits.is_empty() {
            changes.entry(uri).or_default().extend(edits);
        }
    }

    Ok(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

/// Resolve `path` to `(uri, content)` for rename use, preferring the
/// editor's in-memory buffer (via `DocStore`) over disk. Returns
/// `None` when the URI can't be constructed or neither source is
/// readable.
fn read_path(store: &DocStore, path: &Path) -> Option<(Url, String)> {
    let uri = Url::from_file_path(path).ok()?;
    if let Some(doc) = store.get(uri.as_str()) {
        return Some((uri, (*doc.text).clone()));
    }
    let content = fs::read_to_string(path).ok()?;
    Some((uri, content))
}

fn byte_range_to_lsp_range(line: u32, byte_range: &std::ops::Range<usize>) -> Range {
    Range {
        start: Position {
            line,
            character: u32::try_from(byte_range.start).unwrap_or(u32::MAX),
        },
        end: Position {
            line,
            character: u32::try_from(byte_range.end).unwrap_or(u32::MAX),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;

    use lid_core::model::{CitationKind, SpecCitation, SpecFile, SpecLine, SpecStatus};
    use lid_core::{ArrowIndex, Unmapped};

    use super::*;

    /// Build a tempdir containing a spec file and a source file, then
    /// return a `LidRepo` pointing at them.
    fn make_repo_on_disk(
        spec_contents: &str,
        spec_line_for_old: usize,
        source_contents: &str,
        source_line_for_old: usize,
    ) -> (tempfile::TempDir, LidRepo) {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        fs::create_dir_all(root.join("docs/specs")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        let spec_path = root.join("docs/specs/auth-specs.md");
        let source_path = root.join("src/auth.rs");
        fs::write(&spec_path, spec_contents).unwrap();
        fs::write(&source_path, source_contents).unwrap();

        let repo = LidRepo {
            root: root.clone(),
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs: vec![SpecFile {
                path: spec_path,
                specs: vec![SpecLine {
                    id: SpecId::parse("AUTH-001").unwrap(),
                    status: SpecStatus::Implemented,
                    text: "text".into(),
                    line: spec_line_for_old,
                }],
                implementing_artifacts: vec![],
                lld: None,
            }],
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![SpecCitation {
                id: SpecId::parse("AUTH-001").unwrap(),
                file: source_path,
                line: source_line_for_old,
                kind: CitationKind::Code,
            }],
        };
        (dir, repo)
    }

    fn position(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    // ── prepare_rename ─────────────────────────────────────────────

    #[test]
    fn prepare_rename_on_citation_returns_range() {
        let text = "// @spec AUTH-001\n";
        let resp = prepare_rename_at_position(text, position(0, 12)).unwrap();
        match resp {
            PrepareRenameResponse::Range(r) => {
                assert_eq!(r.start.line, 0);
                assert_eq!(r.start.character, 9);
                assert_eq!(r.end.character, 17);
            }
            other => panic!("expected Range, got {other:?}"),
        }
    }

    #[test]
    fn prepare_rename_on_definition_returns_range() {
        let text = "- [x] **AUTH-001**: text\n";
        let resp = prepare_rename_at_position(text, position(0, 10)).unwrap();
        match resp {
            PrepareRenameResponse::Range(r) => {
                assert_eq!(r.start.character, 8);
                assert_eq!(r.end.character, 16);
            }
            other => panic!("expected Range, got {other:?}"),
        }
    }

    #[test]
    fn prepare_rename_outside_spec_returns_none() {
        let text = "fn login() {}\n";
        assert!(prepare_rename_at_position(text, position(0, 5)).is_none());
    }

    // ── rename ─────────────────────────────────────────────────────

    #[test]
    fn rename_edits_definition_and_citation() {
        let spec = "# auth\n\n- [x] **AUTH-001**: text\n";
        let source = "// @spec AUTH-001\nfn login() {}\n";
        let (_dir, repo) = make_repo_on_disk(spec, 3, source, 1);

        let cursor_text = "// @spec AUTH-001\n";
        let store = DocStore::new();
        let edit = rename_at_position(
            &repo,
            &store,
            cursor_text,
            position(0, 12),
            "AUTH-LOGIN-001",
        )
        .unwrap();
        let changes = edit.changes.unwrap();
        assert_eq!(changes.len(), 2, "spec file + source file");

        // Each affected URI has at least one edit, and every edit's
        // new_text is the new name.
        for edits in changes.values() {
            assert!(!edits.is_empty());
            for e in edits {
                assert_eq!(e.new_text, "AUTH-LOGIN-001");
            }
        }
    }

    #[test]
    fn rename_to_same_name_is_a_noop() {
        let spec = "- [x] **AUTH-001**: text\n";
        let (_dir, repo) = make_repo_on_disk(spec, 1, "fn x() {}\n", 1);
        let store = DocStore::new();
        let edit = rename_at_position(
            &repo,
            &store,
            "- [x] **AUTH-001**: text\n",
            position(0, 10),
            "AUTH-001",
        )
        .unwrap();
        assert!(edit.changes.is_none() || edit.changes.as_ref().unwrap().is_empty());
    }

    #[test]
    fn rename_to_invalid_name_returns_error() {
        let spec = "- [x] **AUTH-001**: text\n";
        let (_dir, repo) = make_repo_on_disk(spec, 1, "fn x() {}\n", 1);
        let store = DocStore::new();
        let err = rename_at_position(
            &repo,
            &store,
            "- [x] **AUTH-001**: text\n",
            position(0, 10),
            "lowercase",
        )
        .unwrap_err();
        assert!(matches!(err, RenameError::InvalidNewName { .. }), "{err:?}");
    }

    #[test]
    fn rename_to_existing_id_errors_with_collision() {
        let spec = "- [x] **AUTH-001**: text\n- [ ] **AUTH-002**: text\n";
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        fs::create_dir_all(root.join("docs/specs")).unwrap();
        let spec_path = root.join("docs/specs/auth-specs.md");
        fs::write(&spec_path, spec).unwrap();
        let repo = LidRepo {
            root: root.clone(),
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs: vec![SpecFile {
                path: spec_path,
                specs: vec![
                    SpecLine {
                        id: SpecId::parse("AUTH-001").unwrap(),
                        status: SpecStatus::Implemented,
                        text: "t".into(),
                        line: 1,
                    },
                    SpecLine {
                        id: SpecId::parse("AUTH-002").unwrap(),
                        status: SpecStatus::Open,
                        text: "t".into(),
                        line: 2,
                    },
                ],
                implementing_artifacts: vec![],
                lld: None,
            }],
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
        };
        let store = DocStore::new();
        let err = rename_at_position(&repo, &store, spec, position(0, 10), "AUTH-002").unwrap_err();
        assert!(matches!(err, RenameError::Collision { .. }), "{err:?}");
    }

    #[test]
    fn rename_with_cursor_outside_spec_errors_not_on_spec() {
        let (_dir, repo) = make_repo_on_disk("- [x] **AUTH-001**: text\n", 1, "fn x() {}\n", 1);
        let store = DocStore::new();
        let err = rename_at_position(&repo, &store, "fn x() {}\n", position(0, 2), "AUTH-002")
            .unwrap_err();
        assert!(matches!(err, RenameError::NotOnSpec), "{err:?}");
    }

    #[test]
    fn rename_edit_byte_ranges_match_old_spec_id_spans() {
        let spec = "# auth\n\n- [x] **AUTH-001**: text\n";
        let source = "// @spec AUTH-001\nfn login() {}\n";
        let (_dir, repo) = make_repo_on_disk(spec, 3, source, 1);

        let store = DocStore::new();
        let edit = rename_at_position(
            &repo,
            &store,
            "// @spec AUTH-001\n",
            position(0, 12),
            "AUTH-LOGIN-001",
        )
        .unwrap();
        let changes = edit.changes.unwrap();
        for (uri, edits) in &changes {
            assert_eq!(edits.len(), 1, "one edit per affected URI");
            let r = &edits[0].range;
            if uri.path().ends_with("auth-specs.md") {
                // `**AUTH-001**` on line 3 → bytes 8..16.
                assert_eq!(r.start.line, 2);
                assert_eq!(r.start.character, 8);
                assert_eq!(r.end.character, 16);
            } else {
                // `// @spec AUTH-001` on line 1 → bytes 9..17.
                assert_eq!(r.start.line, 0);
                assert_eq!(r.start.character, 9);
                assert_eq!(r.end.character, 17);
            }
        }
    }

    #[test]
    fn rename_reads_buffer_content_when_url_is_in_store() {
        // Set up: disk says AUTH-001 is on line 3. The user has
        // since edited the buffer in their editor, shifting AUTH-001
        // to line 5 (added two blank lines above). The DocStore
        // tracks the buffer content. Rename must use the buffer
        // offsets, not the stale disk offsets.
        let on_disk = "# auth\n\n- [x] **AUTH-001**: text\n";
        let (_dir, repo) = make_repo_on_disk(on_disk, 3, "fn x() {}\n", 1);

        let spec_path = &repo.specs[0].path;
        let buffer_text = "# auth\n\nan extra paragraph\n\n- [x] **AUTH-001**: text\n";
        let store = DocStore::new();
        let _ = store.upsert(
            Url::from_file_path(spec_path).unwrap().as_str().to_owned(),
            buffer_text.to_owned(),
            1,
        );

        // Pretend the cursor is on AUTH-001 in some other file
        // (citation text). The rename will resolve to the spec file
        // edit and must read the BUFFER content for it.
        let cursor_text = "// @spec AUTH-001\n";
        let edit = rename_at_position(
            &repo,
            &store,
            cursor_text,
            position(0, 12),
            "AUTH-LOGIN-001",
        )
        .unwrap();
        let changes = edit.changes.unwrap();
        let spec_uri = Url::from_file_path(spec_path).unwrap();
        let spec_edits = changes
            .get(&spec_uri)
            .expect("expected an edit for the spec file");
        assert_eq!(spec_edits.len(), 1);
        // In the BUFFER content, AUTH-001 lives on line 5 (0-based:
        // 4). If the rename had used disk content, the edit would
        // have targeted line 2 (0-based), corrupting the buffer.
        assert_eq!(spec_edits[0].range.start.line, 4);
        assert_eq!(spec_edits[0].range.start.character, 8);
        assert_eq!(spec_edits[0].range.end.character, 16);
    }
}
