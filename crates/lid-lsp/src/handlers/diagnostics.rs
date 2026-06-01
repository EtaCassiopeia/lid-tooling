//! `textDocument/publishDiagnostics` for the active buffer.
//!
//! Two diagnostic producers:
//!
//! * `diagnostics_for_buffer` — source / test files. Flags reverse
//!   orphans: `@spec SPEC-ID` citations whose ID isn't defined in any
//!   spec file.
//!
//! * `diagnostics_for_spec_buffer` — `*-specs.md` files. Flags coverage
//!   gaps (`[x]` spec with no `@spec` citation) and status mismatches
//!   (`[ ]`/`[D]` spec that already has citations).

use std::collections::BTreeSet;

use lid_core::model::SpecStatus;
use lid_core::parse::markdown::find_spec_line_match;
use lid_core::parse::source::find_citations_in_line;
use lid_core::{LidRepo, SpecId};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

/// Compute diagnostics for `text` against the repository's known spec IDs.
///
/// Empty result when the buffer has no `@spec` citations or every
/// cited ID is defined.
#[must_use]
pub fn diagnostics_for_buffer(repo: &LidRepo, text: &str) -> Vec<Diagnostic> {
    let known: BTreeSet<&SpecId> = repo
        .specs
        .iter()
        .flat_map(|f| f.specs.iter().map(|s| &s.id))
        .collect();

    let mut out = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let line_num = u32::try_from(line_idx).unwrap_or(u32::MAX);
        for c in find_citations_in_line(line) {
            if known.contains(&c.id) {
                continue;
            }
            out.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_num,
                        character: u32::try_from(c.byte_range.start).unwrap_or(u32::MAX),
                    },
                    end: Position {
                        line: line_num,
                        character: u32::try_from(c.byte_range.end).unwrap_or(u32::MAX),
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("reverse-orphan".into())),
                source: Some(env!("CARGO_PKG_NAME").to_owned()),
                message: format!(
                    "`@spec {}` references a spec ID that is not defined in any spec file",
                    c.id
                ),
                ..Diagnostic::default()
            });
        }
    }
    out
}

/// Compute diagnostics for a spec file (`*-specs.md`) buffer.
///
/// Scans for spec lines (`- [x] **ID**: …`) and flags two mismatches:
/// * `[x]` (implemented) with no `@spec` citations anywhere in the repo
///   → `Error` / code `coverage`.
/// * `[ ]` or `[D]` with one or more citations
///   → `Warning` / code `spec-status`.
///
/// The diagnostic range covers the bare spec ID (the text inside `**…**`)
/// so the squiggly appears precisely under the identifier, matching the
/// position that hover and go-to-references already highlight.
#[must_use]
pub fn diagnostics_for_spec_buffer(repo: &LidRepo, text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        let line_num = u32::try_from(line_idx).unwrap_or(u32::MAX);

        let Some(m) = find_spec_line_match(line) else {
            continue;
        };

        let citation_count = repo.citations.iter().filter(|c| c.id == m.id).count();
        let start_char = u32::try_from(m.id_byte_range.start).unwrap_or(u32::MAX);
        let end_char = u32::try_from(m.id_byte_range.end).unwrap_or(u32::MAX);
        let range = Range {
            start: Position {
                line: line_num,
                character: start_char,
            },
            end: Position {
                line: line_num,
                character: end_char,
            },
        };

        match m.status {
            SpecStatus::Implemented if citation_count == 0 => {
                out.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("coverage".into())),
                    source: Some(env!("CARGO_PKG_NAME").to_owned()),
                    message: format!(
                        "`{}` is marked implemented (`[x]`) but has no `@spec` citations \
                         in any source or test file",
                        m.id
                    ),
                    ..Diagnostic::default()
                });
            }
            SpecStatus::Open | SpecStatus::Deferred if citation_count > 0 => {
                let marker = if m.status == SpecStatus::Open {
                    "[ ]"
                } else {
                    "[D]"
                };
                out.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::String("spec-status".into())),
                    source: Some(env!("CARGO_PKG_NAME").to_owned()),
                    message: format!(
                        "`{}` is marked `{marker}` but has {citation_count} `@spec` \
                         citation{} — mark it `[x]` or remove the annotations",
                        m.id,
                        if citation_count == 1 { "" } else { "s" },
                    ),
                    ..Diagnostic::default()
                });
            }
            _ => {}
        }
    }

    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{CitationKind, SpecCitation, SpecFile, SpecLine, SpecStatus};
    use lid_core::{ArrowIndex, Unmapped};

    use super::*;

    fn repo_with_spec(id: &str) -> LidRepo {
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
                    id: SpecId::parse(id).unwrap(),
                    status: SpecStatus::Implemented,
                    text: "text".into(),
                    line: 1,
                }],
                implementing_artifacts: vec![],
                lld: None,
                prefix: None,
            }],
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
            decision_docs: vec![],
        }
    }

    #[test]
    fn known_spec_citation_yields_no_diagnostic() {
        let repo = repo_with_spec("AUTH-001");
        let diags = diagnostics_for_buffer(&repo, "// @spec AUTH-001\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn unknown_spec_citation_yields_an_error_diagnostic() {
        let repo = repo_with_spec("AUTH-001");
        let diags = diagnostics_for_buffer(&repo, "// @spec UNKNOWN-001\nfn x() {}\n");
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
        assert!(d.message.contains("UNKNOWN-001"));
        assert_eq!(
            d.code,
            Some(NumberOrString::String("reverse-orphan".into()))
        );
        assert_eq!(d.source.as_deref(), Some(env!("CARGO_PKG_NAME")));
    }

    #[test]
    fn diagnostic_range_covers_the_offending_spec_id() {
        let repo = repo_with_spec("AUTH-001");
        let diags = diagnostics_for_buffer(&repo, "// @spec UNKNOWN-001\n");
        let r = diags[0].range;
        // "UNKNOWN-001" sits at bytes 9..20 in `// @spec UNKNOWN-001`.
        assert_eq!(r.start.line, 0);
        assert_eq!(r.start.character, 9);
        assert_eq!(r.end.character, 20);
    }

    #[test]
    fn buffer_without_at_spec_yields_no_diagnostics() {
        let repo = repo_with_spec("AUTH-001");
        let diags = diagnostics_for_buffer(&repo, "const SPECS = ['AUTH-001'];\n");
        assert!(diags.is_empty());
    }

    #[test]
    fn one_diagnostic_per_offending_citation() {
        let repo = repo_with_spec("AUTH-001");
        let text = "// @spec UNKNOWN-001\nfn a() {}\n// @spec ALSO-MISSING-002\n";
        let diags = diagnostics_for_buffer(&repo, text);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].range.start.line, 0);
        assert_eq!(diags[1].range.start.line, 2);
    }

    #[test]
    fn mixed_known_and_unknown_only_flags_unknown() {
        let repo = repo_with_spec("AUTH-001");
        let text = "// @spec AUTH-001, UNKNOWN-002\n";
        let diags = diagnostics_for_buffer(&repo, text);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("UNKNOWN-002"));
    }

    // ── diagnostics_for_spec_buffer ──────────────────────────────────────────

    fn repo_with_spec_and_citations(
        id: &str,
        status: SpecStatus,
        citation_files: &[&str],
    ) -> LidRepo {
        let root = PathBuf::from("/repo");
        let citations = citation_files
            .iter()
            .map(|p| SpecCitation {
                id: SpecId::parse(id).unwrap(),
                file: root.join(p),
                line: 1,
                kind: CitationKind::Test,
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
                path: root.join("docs/intent/auth/auth-specs.md"),
                specs: vec![SpecLine {
                    id: SpecId::parse(id).unwrap(),
                    status,
                    text: "text".into(),
                    line: 1,
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

    #[test]
    fn implemented_spec_with_citation_yields_no_diagnostic() {
        let repo = repo_with_spec_and_citations(
            "AUTH-001",
            SpecStatus::Implemented,
            &["tests/auth_test.rs"],
        );
        let text = "- [x] **AUTH-001**: When the user logs in\n";
        assert!(diagnostics_for_spec_buffer(&repo, text).is_empty());
    }

    #[test]
    fn implemented_spec_without_citation_yields_error() {
        let repo = repo_with_spec_and_citations("AUTH-001", SpecStatus::Implemented, &[]);
        let text = "- [x] **AUTH-001**: When the user logs in\n";
        let diags = diagnostics_for_spec_buffer(&repo, text);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(d.code, Some(NumberOrString::String("coverage".into())));
        assert!(d.message.contains("AUTH-001"));
        assert!(d.message.contains("[x]"));
    }

    #[test]
    fn coverage_diagnostic_range_covers_bare_id() {
        let repo = repo_with_spec_and_citations("AUTH-001", SpecStatus::Implemented, &[]);
        let text = "- [x] **AUTH-001**: text\n";
        let diags = diagnostics_for_spec_buffer(&repo, text);
        let r = diags[0].range;
        // "AUTH-001" starts at byte 8 in `- [x] **AUTH-001**: …` (after `**`)
        assert_eq!(r.start.line, 0);
        assert_eq!(r.start.character, 8);
        assert_eq!(r.end.character, 16);
    }

    #[test]
    fn open_spec_with_citation_yields_warning() {
        let repo =
            repo_with_spec_and_citations("AUTH-001", SpecStatus::Open, &["tests/auth_test.rs"]);
        let text = "- [ ] **AUTH-001**: When the user logs in\n";
        let diags = diagnostics_for_spec_buffer(&repo, text);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(d.code, Some(NumberOrString::String("spec-status".into())));
        assert!(d.message.contains("AUTH-001"));
        assert!(d.message.contains("[ ]"));
    }

    #[test]
    fn deferred_spec_with_citation_yields_warning() {
        let repo =
            repo_with_spec_and_citations("AUTH-001", SpecStatus::Deferred, &["tests/auth_test.rs"]);
        let text = "- [D] **AUTH-001**: When the user logs in\n";
        let diags = diagnostics_for_spec_buffer(&repo, text);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
        assert!(diags[0].message.contains("[D]"));
    }

    #[test]
    fn open_spec_without_citation_yields_no_diagnostic() {
        let repo = repo_with_spec_and_citations("AUTH-001", SpecStatus::Open, &[]);
        let text = "- [ ] **AUTH-001**: When the user logs in\n";
        assert!(diagnostics_for_spec_buffer(&repo, text).is_empty());
    }

    #[test]
    fn non_spec_lines_are_ignored() {
        let repo = repo_with_spec_and_citations("AUTH-001", SpecStatus::Implemented, &[]);
        let text = "# EARS Specs: auth\n\n## Section\n\nsome prose\n";
        assert!(diagnostics_for_spec_buffer(&repo, text).is_empty());
    }

    #[test]
    fn spec_diagnostic_correct_line_number() {
        let repo = repo_with_spec_and_citations("AUTH-001", SpecStatus::Implemented, &[]);
        let text = "# Auth specs\n\n- [x] **AUTH-001**: text\n";
        let diags = diagnostics_for_spec_buffer(&repo, text);
        assert_eq!(diags[0].range.start.line, 2);
    }
}
