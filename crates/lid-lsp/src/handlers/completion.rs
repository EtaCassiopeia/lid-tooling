//! `textDocument/completion` for `@spec ` contexts.
//!
//! Triggers when the cursor sits after `@spec ` or after a comma in a
//! multi-ID citation. Offers every spec ID in the repository whose
//! prefix matches what the user has typed, sorted by ID.
//!
//! Outside `@spec` contexts the handler returns `None` so the editor
//! falls back to its default completion source.

use lid_core::LidRepo;
use lid_core::model::{SpecLine, SpecStatus};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, MarkupContent,
    MarkupKind, Position,
};

/// Compute the completion response for the cursor position.
///
/// Returns:
/// * `Some(CompletionResponse::Array(items))` when the cursor is in
///   `@spec` context — `items` lists every spec ID whose prefix matches
///   what the user is typing.
/// * `None` otherwise, so the editor's default completion takes over.
pub fn completions_at_position(
    repo: &LidRepo,
    text: &str,
    position: Position,
) -> Option<CompletionResponse> {
    let line_idx = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_idx)?;
    let cursor_byte = usize::try_from(position.character).ok()?;
    let cursor_byte = cursor_byte.min(line.len());

    let typed_prefix = at_spec_prefix(&line[..cursor_byte])?;

    let mut items: Vec<CompletionItem> = repo
        .specs
        .iter()
        .flat_map(|f| f.specs.iter())
        .filter(|s| s.id.as_str().starts_with(typed_prefix))
        .map(make_completion_item)
        .collect();
    items.sort_by(|a, b| a.label.cmp(&b.label));

    Some(CompletionResponse::Array(items))
}

/// Returns `Some(prefix)` when the slice of the line up to the cursor
/// is positioned inside a `@spec ` citation context. The prefix is the
/// chars the user has typed so far (may be empty).
fn at_spec_prefix(prefix_line: &str) -> Option<&str> {
    // Find the rightmost `@spec` token.
    let at_spec_start = prefix_line.rfind("@spec")?;
    let after = &prefix_line[at_spec_start + "@spec".len()..];

    // Require a whitespace separator immediately after `@spec`, unless
    // the keyword sits at the very end of the line (cursor on the
    // space the user is about to type, but `@spec\b`-only would be
    // legitimately `@specification` or similar — bail out).
    let mut chars = after.chars();
    let first = chars.next();
    let is_word_continuation = first.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    if is_word_continuation {
        return None;
    }

    // Everything between `@spec` and the cursor must be valid chars for
    // a comma-separated spec-ID list: whitespace, commas, and spec-ID
    // tokens (uppercase letters, digits, hyphens).
    if !after.chars().all(|c| {
        c.is_whitespace() || c == ',' || c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'
    }) {
        return None;
    }

    // The typed prefix is the trailing run of spec-ID chars after the
    // last whitespace or comma.
    let trailing = after
        .rsplit(|c: char| c.is_whitespace() || c == ',')
        .next()
        .unwrap_or("");

    Some(trailing)
}

fn make_completion_item(spec: &SpecLine) -> CompletionItem {
    let status = match spec.status {
        SpecStatus::Implemented => "[x] implemented",
        SpecStatus::Open => "[ ] open",
        SpecStatus::Deferred => "[D] deferred",
    };
    CompletionItem {
        label: spec.id.to_string(),
        kind: Some(CompletionItemKind::REFERENCE),
        detail: Some(status.to_owned()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("**`{}`** — *{}*\n\n> {}", spec.id, status, spec.text),
        })),
        ..CompletionItem::default()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::model::{SpecFile, SpecLine, SpecStatus};
    use lid_core::{ArrowIndex, SpecId, Unmapped};

    use super::*;

    fn repo_with_specs(ids: &[&str]) -> LidRepo {
        let root = PathBuf::from("/repo");
        let specs = vec![SpecFile {
            path: root.join("docs/specs/auth-specs.md"),
            specs: ids
                .iter()
                .enumerate()
                .map(|(i, id)| SpecLine {
                    id: SpecId::parse(id).unwrap(),
                    status: SpecStatus::Implemented,
                    text: format!("text for {id}"),
                    line: i + 1,
                })
                .collect(),
            implementing_artifacts: vec![],
            lld: None,
        }];
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
        }
    }

    fn position(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    fn labels(resp: &CompletionResponse) -> Vec<String> {
        match resp {
            CompletionResponse::Array(items) => items.iter().map(|i| i.label.clone()).collect(),
            CompletionResponse::List(list) => list.items.iter().map(|i| i.label.clone()).collect(),
        }
    }

    #[test]
    fn after_at_spec_space_offers_all_specs() {
        let repo = repo_with_specs(&["AUTH-001", "BILL-002"]);
        let text = "// @spec ";
        let resp = completions_at_position(&repo, text, position(0, 9)).unwrap();
        assert_eq!(labels(&resp), vec!["AUTH-001", "BILL-002"]);
    }

    #[test]
    fn prefix_filters_by_start() {
        let repo = repo_with_specs(&["AUTH-001", "AUTH-002", "BILL-001"]);
        let text = "// @spec AU";
        let resp = completions_at_position(&repo, text, position(0, 11)).unwrap();
        let labels = labels(&resp);
        assert_eq!(labels, vec!["AUTH-001", "AUTH-002"]);
    }

    #[test]
    fn full_prefix_returns_single_exact_match() {
        let repo = repo_with_specs(&["AUTH-001", "AUTH-002"]);
        let text = "// @spec AUTH-001";
        let resp = completions_at_position(&repo, text, position(0, 17)).unwrap();
        assert_eq!(labels(&resp), vec!["AUTH-001"]);
    }

    #[test]
    fn after_comma_offers_all_specs_again() {
        let repo = repo_with_specs(&["AUTH-001", "BILL-002"]);
        let text = "// @spec AUTH-001, ";
        let resp = completions_at_position(&repo, text, position(0, 19)).unwrap();
        assert_eq!(labels(&resp), vec!["AUTH-001", "BILL-002"]);
    }

    #[test]
    fn after_comma_with_prefix_filters() {
        let repo = repo_with_specs(&["AUTH-001", "AUTH-002", "BILL-003"]);
        let text = "// @spec AUTH-001, B";
        let resp = completions_at_position(&repo, text, position(0, 20)).unwrap();
        assert_eq!(labels(&resp), vec!["BILL-003"]);
    }

    #[test]
    fn outside_at_spec_context_returns_none() {
        let repo = repo_with_specs(&["AUTH-001"]);
        let text = "fn login() {}";
        assert!(completions_at_position(&repo, text, position(0, 5)).is_none());
    }

    #[test]
    fn pseudo_keyword_does_not_trigger() {
        let repo = repo_with_specs(&["AUTH-001"]);
        // `@specification` looks like `@spec` followed by `ification`,
        // which is a word continuation — must not trigger completion.
        let text = "// @specification of AUTH-";
        assert!(completions_at_position(&repo, text, position(0, 26)).is_none());
    }

    #[test]
    fn lowercase_after_at_spec_does_not_trigger() {
        let repo = repo_with_specs(&["AUTH-001"]);
        // After @spec, the typed prefix must be valid spec-ID chars
        // (uppercase / digit / hyphen). Lowercase means we're not in
        // a citation list anymore.
        let text = "// @spec foo";
        assert!(completions_at_position(&repo, text, position(0, 12)).is_none());
    }

    #[test]
    fn completion_item_carries_status_and_docs() {
        let repo = repo_with_specs(&["AUTH-001"]);
        let text = "// @spec ";
        let resp = completions_at_position(&repo, text, position(0, 9)).unwrap();
        let items = match &resp {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => &list.items,
        };
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.label, "AUTH-001");
        assert_eq!(item.kind, Some(CompletionItemKind::REFERENCE));
        assert!(item.detail.as_deref().unwrap().contains("implemented"));
        match &item.documentation {
            Some(Documentation::MarkupContent(m)) => {
                assert!(m.value.contains("AUTH-001"));
                assert!(m.value.contains("text for AUTH-001"));
            }
            other => panic!("expected markup documentation, got {other:?}"),
        }
    }

    #[test]
    fn empty_repo_returns_empty_array() {
        let repo = repo_with_specs(&[]);
        let text = "// @spec ";
        let resp = completions_at_position(&repo, text, position(0, 9)).unwrap();
        assert_eq!(labels(&resp), Vec::<String>::new());
    }
}
