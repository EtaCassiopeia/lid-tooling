//! `textDocument/publishDiagnostics` for the active buffer.
//!
//! Today's diagnostics surface a single class of issue: reverse
//! orphans — `@spec SPEC-ID` references in the current buffer whose
//! ID isn't defined in any spec file the repo loaded. That's the
//! issue an editor user wants to see *now*, before saving, as they
//! mistype a spec ID or paste an annotation forgetting to add the
//! corresponding spec.
//!
//! Other diagnostic classes (orphaned spec files, broken arrow
//! references, etc.) live behind `lidc check`'s richer output and
//! reach the LSP in later commits.

use std::collections::BTreeSet;

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{SpecFile, SpecLine, SpecStatus};
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
            }],
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
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
}
