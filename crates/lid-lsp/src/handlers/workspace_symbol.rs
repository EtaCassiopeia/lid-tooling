//! `workspace/symbol` for spec IDs.
//!
//! Lists every spec defined in the repository as an LSP
//! `SymbolInformation`. The optional `query` filters by
//! case-insensitive substring against the spec ID — empty query
//! returns everything.

use lid_core::LidRepo;
use tower_lsp::lsp_types::{Location, Position, Range, SymbolInformation, SymbolKind, Url};

/// Build the workspace-symbol response.
///
/// Sorted by spec ID so the editor's quick-open list is stable.
pub fn list_symbols(repo: &LidRepo, query: &str) -> Vec<SymbolInformation> {
    let needle = query.to_ascii_uppercase();
    let mut symbols: Vec<SymbolInformation> = Vec::new();

    for spec_file in &repo.specs {
        let Ok(uri) = Url::from_file_path(&spec_file.path) else {
            continue;
        };
        let container = spec_file
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_owned);

        for line in &spec_file.specs {
            if !needle.is_empty() && !line.id.as_str().contains(&needle) {
                continue;
            }
            let target = u32::try_from(line.line.saturating_sub(1)).unwrap_or(0);
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: line.id.to_string(),
                kind: SymbolKind::CONSTANT,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
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
                },
                container_name: container.clone(),
            });
        }
    }

    symbols.sort_by(|a, b| a.name.cmp(&b.name));
    symbols
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{SpecFile, SpecLine, SpecStatus};
    use lid_core::{ArrowIndex, SpecId, Unmapped};

    use super::*;

    fn root() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("C:\\repo")
        } else {
            PathBuf::from("/repo")
        }
    }

    fn make_repo(specs_by_file: &[(&str, &[&str])]) -> LidRepo {
        let root = root();
        let specs = specs_by_file
            .iter()
            .map(|(file_rel, ids)| SpecFile {
                path: root.join(file_rel),
                specs: ids
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
                prefix: None,
            })
            .collect();
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
            citations: vec![],
            decision_docs: vec![],
        }
    }

    #[test]
    fn empty_query_returns_every_spec_sorted() {
        let repo = make_repo(&[
            ("docs/specs/auth-specs.md", &["AUTH-002", "AUTH-001"]),
            ("docs/specs/billing-specs.md", &["BILL-001"]),
        ]);
        let symbols = list_symbols(&repo, "");
        let names: Vec<_> = symbols.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names, vec!["AUTH-001", "AUTH-002", "BILL-001"]);
    }

    #[test]
    fn query_filters_case_insensitive_substring() {
        let repo = make_repo(&[
            ("docs/specs/auth-specs.md", &["AUTH-001", "AUTH-002"]),
            ("docs/specs/billing-specs.md", &["BILL-001"]),
        ]);
        let symbols = list_symbols(&repo, "auth");
        let names: Vec<_> = symbols.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names, vec!["AUTH-001", "AUTH-002"]);
    }

    #[test]
    fn query_matches_middle_of_id() {
        let repo = make_repo(&[("docs/specs/auth-specs.md", &["AUTH-001", "BILL-001"])]);
        let symbols = list_symbols(&repo, "-00");
        // Both end in `-001`, so substring match catches both.
        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn symbols_carry_container_name() {
        let repo = make_repo(&[("docs/specs/auth-specs.md", &["AUTH-001"])]);
        let symbols = list_symbols(&repo, "");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].container_name.as_deref(), Some("auth-specs.md"));
    }

    #[test]
    fn symbol_location_points_at_correct_line() {
        let repo = make_repo(&[("docs/specs/auth-specs.md", &["AUTH-001", "AUTH-002"])]);
        let symbols = list_symbols(&repo, "");
        // SpecLine.line is 1-based (i+1 in the helper, so AUTH-001 = line 1).
        assert_eq!(symbols[0].location.range.start.line, 0);
        assert_eq!(symbols[1].location.range.start.line, 1);
    }

    #[test]
    fn empty_repo_returns_empty_list() {
        let repo = make_repo(&[]);
        assert!(list_symbols(&repo, "").is_empty());
    }
}
