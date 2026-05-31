//! `textDocument/completion` for `@spec` and arrow-doc section-heading contexts.
//!
//! Two completion triggers are supported:
//!
//! 1. **`@spec` citation** — triggers after `@spec ` or a comma in a
//!    multi-ID citation; offers every matching spec ID sorted by ID.
//!
//! 2. **Arrow-doc section heading** — triggers when the cursor is on a
//!    `### ` line inside the `## References` block of an arrow doc;
//!    offers the five canonical subsection names (`HLD`, `LLD`, `EARS`,
//!    `Tests`, `Code`). Client-side prefix filtering applies.
//!
//! Outside both contexts the handler returns `None` so the editor falls
//! back to its default completion source.

use lid_core::LidRepo;
use lid_core::model::{SpecLine, SpecStatus};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, MarkupContent,
    MarkupKind, Position, Url,
};

/// Compute the completion response for the cursor position.
///
/// Returns:
/// * `Some(CompletionResponse::Array(items))` with five section names when the
///   cursor is on a `### ` line inside `## References` of an arrow doc.
/// * `Some(CompletionResponse::Array(items))` with matching spec IDs when the
///   cursor is in an `@spec` citation context.
/// * `None` otherwise, so the editor's default completion takes over.
pub fn completions_at_position(
    repo: &LidRepo,
    uri: &str,
    text: &str,
    position: Position,
) -> Option<CompletionResponse> {
    let line_idx = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_idx)?;
    let cursor_byte = usize::try_from(position.character).ok()?;
    let cursor_byte = cursor_byte.min(line.len());
    let prefix = &line[..cursor_byte];

    if is_arrow_doc(repo, uri) && prefix.starts_with("### ") && in_references_block(text, line_idx)
    {
        return Some(CompletionResponse::Array(section_heading_items()));
    }

    let typed_prefix = at_spec_prefix(prefix)?;
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

fn is_arrow_doc(repo: &LidRepo, uri: &str) -> bool {
    let Ok(url) = Url::parse(uri) else {
        return false;
    };
    let Ok(path) = url.to_file_path() else {
        return false;
    };
    path.starts_with(repo.root.join("docs").join("arrows"))
        && path.extension().is_some_and(|e| e == "md")
}

fn in_references_block(text: &str, line_idx: usize) -> bool {
    for line in text
        .lines()
        .take(line_idx)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if let Some(heading) = line.strip_prefix("## ") {
            return heading.trim() == "References";
        }
    }
    false
}

fn section_heading_items() -> Vec<CompletionItem> {
    [
        ("HLD", "High-level design — links to the architecture doc"),
        (
            "LLD",
            "Low-level design — links to the implementation intent doc",
        ),
        (
            "EARS",
            "EARS spec file — links to the behavioural requirements",
        ),
        ("Tests", "Test and eval fixtures"),
        ("Code", "Source files and skill prompts"),
    ]
    .into_iter()
    .map(|(name, doc)| CompletionItem {
        label: name.to_owned(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(format!("### {name}")),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.to_owned(),
        })),
        ..CompletionItem::default()
    })
    .collect()
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
            prefix: None,
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
        let resp = completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 9))
            .unwrap();
        assert_eq!(labels(&resp), vec!["AUTH-001", "BILL-002"]);
    }

    #[test]
    fn prefix_filters_by_start() {
        let repo = repo_with_specs(&["AUTH-001", "AUTH-002", "BILL-001"]);
        let text = "// @spec AU";
        let resp =
            completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 11))
                .unwrap();
        let labels = labels(&resp);
        assert_eq!(labels, vec!["AUTH-001", "AUTH-002"]);
    }

    #[test]
    fn full_prefix_returns_single_exact_match() {
        let repo = repo_with_specs(&["AUTH-001", "AUTH-002"]);
        let text = "// @spec AUTH-001";
        let resp =
            completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 17))
                .unwrap();
        assert_eq!(labels(&resp), vec!["AUTH-001"]);
    }

    #[test]
    fn after_comma_offers_all_specs_again() {
        let repo = repo_with_specs(&["AUTH-001", "BILL-002"]);
        let text = "// @spec AUTH-001, ";
        let resp =
            completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 19))
                .unwrap();
        assert_eq!(labels(&resp), vec!["AUTH-001", "BILL-002"]);
    }

    #[test]
    fn after_comma_with_prefix_filters() {
        let repo = repo_with_specs(&["AUTH-001", "AUTH-002", "BILL-003"]);
        let text = "// @spec AUTH-001, B";
        let resp =
            completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 20))
                .unwrap();
        assert_eq!(labels(&resp), vec!["BILL-003"]);
    }

    #[test]
    fn outside_at_spec_context_returns_none() {
        let repo = repo_with_specs(&["AUTH-001"]);
        let text = "fn login() {}";
        assert!(
            completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 5))
                .is_none()
        );
    }

    #[test]
    fn pseudo_keyword_does_not_trigger() {
        let repo = repo_with_specs(&["AUTH-001"]);
        // `@specification` looks like `@spec` followed by `ification`,
        // which is a word continuation — must not trigger completion.
        let text = "// @specification of AUTH-";
        assert!(
            completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 26))
                .is_none()
        );
    }

    #[test]
    fn lowercase_after_at_spec_does_not_trigger() {
        let repo = repo_with_specs(&["AUTH-001"]);
        // After @spec, the typed prefix must be valid spec-ID chars
        // (uppercase / digit / hyphen). Lowercase means we're not in
        // a citation list anymore.
        let text = "// @spec foo";
        assert!(
            completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 12))
                .is_none()
        );
    }

    #[test]
    fn completion_item_carries_status_and_docs() {
        let repo = repo_with_specs(&["AUTH-001"]);
        let text = "// @spec ";
        let resp = completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 9))
            .unwrap();
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
        let resp = completions_at_position(&repo, "file:///repo/src/auth.rs", text, position(0, 9))
            .unwrap();
        assert_eq!(labels(&resp), Vec::<String>::new());
    }

    // ── Section heading completions ────────────────────────────────────

    const ARROW_URI: &str = "file:///repo/docs/arrows/auth.md";
    const NON_ARROW_URI: &str = "file:///repo/docs/intent/auth/auth-design.md";

    const ARROW_TEXT: &str = "\
# Arrow: auth

## Status

ok

## References

### ";

    #[test]
    fn section_completions_trigger_on_triple_hash_in_references() {
        let repo = repo_with_specs(&[]);
        // Cursor after "### " on line 8 (0-indexed)
        let resp = completions_at_position(&repo, ARROW_URI, ARROW_TEXT, position(8, 4)).unwrap();
        assert_eq!(labels(&resp).len(), 5);
    }

    #[test]
    fn section_completions_return_five_items_with_docs() {
        let repo = repo_with_specs(&[]);
        let resp = completions_at_position(&repo, ARROW_URI, ARROW_TEXT, position(8, 4)).unwrap();
        let items = match &resp {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(l) => &l.items,
        };
        assert_eq!(items.len(), 5);
        let expected = ["HLD", "LLD", "EARS", "Tests", "Code"];
        for (item, name) in items.iter().zip(expected) {
            assert_eq!(item.label, name);
            assert_eq!(item.kind, Some(CompletionItemKind::KEYWORD));
            assert_eq!(item.detail.as_deref(), Some(format!("### {name}").as_str()));
            assert!(item.documentation.is_some());
        }
    }

    #[test]
    fn section_completions_do_not_trigger_in_non_arrow_doc() {
        let repo = repo_with_specs(&[]);
        // Same content but URI is not under docs/arrows/ → falls through to @spec (None)
        assert!(
            completions_at_position(&repo, NON_ARROW_URI, ARROW_TEXT, position(8, 4)).is_none()
        );
    }

    #[test]
    fn section_completions_do_not_trigger_outside_references_block() {
        let repo = repo_with_specs(&[]);
        let text = "\
## Architecture

### ";
        // Nearest H2 is "Architecture", not "References"
        assert!(completions_at_position(&repo, ARROW_URI, text, position(2, 4)).is_none());
    }

    #[test]
    fn section_completions_do_not_trigger_after_later_h2_overrides_references() {
        let repo = repo_with_specs(&[]);
        let text = "\
## References

### HLD

- docs/hld.md

## Architecture

### ";
        // Nearest H2 before line 8 is "Architecture" — not inside References anymore
        assert!(completions_at_position(&repo, ARROW_URI, text, position(8, 4)).is_none());
    }

    #[test]
    fn section_completions_do_not_trigger_on_double_hash() {
        let repo = repo_with_specs(&[]);
        let text = "\
## References

## ";
        // "## " is an H2 line, not H3 — no section completions
        assert!(completions_at_position(&repo, ARROW_URI, text, position(2, 3)).is_none());
    }

    #[test]
    fn in_references_block_false_with_no_preceding_h2() {
        assert!(!in_references_block("### ", 0));
        assert!(!in_references_block("some prose\n### ", 1));
    }
}
