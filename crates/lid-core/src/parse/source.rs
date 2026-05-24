//! Scan source files for `@spec` citations.
//!
//! Walks a directory tree honouring `.gitignore` (via the `ignore` crate)
//! and extracts every `@spec SPEC-ID, SPEC-ID, …` reference. Matches the
//! regex strategy of the upstream Node reference implementation
//! (`coherence-check.mjs`) so behaviour is consistent across tools:
//! a two-stage filter — line must contain the `@spec` keyword, then any
//! spec-shaped uppercase identifier on that line counts.
//!
//! Because the regex engine in `regex` has no negative lookahead, we
//! anchor the candidate match with `\b…\b` word boundaries and post-
//! filter by requiring at least one digit. That excludes both `A-Z`
//! pseudo-matches and accidental CamelCase hits such as `GSI-ByX`.

use std::path::Path;
use std::sync::LazyLock;

use ignore::WalkBuilder;
use regex::Regex;

use crate::model::{CitationKind, SpecCitation, SpecId};

/// Word-boundary marker for the `@spec` keyword. The trailing `\b` keeps
/// us from matching `@specification` or `@speculative`.
#[allow(clippy::expect_used)] // constant pattern; compilation is infallible
static AT_SPEC_KEYWORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@spec\b").expect("AT_SPEC_KEYWORD_RE compiles"));

/// Unanchored counterpart to the spec-ID pattern in `SpecId::parse`.
///
/// Word-bounded on both sides so mid-identifier hits in CamelCase
/// (`GSI-ByReminderDue`) are excluded. The digit-presence post-filter
/// happens after the match.
#[allow(clippy::expect_used)] // constant pattern; compilation is infallible
static SPEC_ID_IN_TEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+\b").expect("SPEC_ID_IN_TEXT_RE compiles")
});

/// Walk `root` and collect every `@spec` citation in source files.
///
/// Honours `.gitignore` and skips files that aren't valid UTF-8
/// (binaries, byte-sequences). Errors during walking are silently
/// ignored — this is best-effort discovery, not validation.
#[must_use]
pub fn scan_path(root: &Path) -> Vec<SpecCitation> {
    let mut out = Vec::new();
    for result in WalkBuilder::new(root).build() {
        let Ok(entry) = result else { continue };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        scan_content(&content, path, &mut out);
    }
    out
}

/// Scan a single in-memory string and append any citations to `out`.
pub fn scan_content(content: &str, path: &Path, out: &mut Vec<SpecCitation>) {
    let kind = classify_path(path);
    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        for citation in find_citations_in_line(line) {
            out.push(SpecCitation {
                id: citation.id,
                file: path.to_path_buf(),
                line: line_no,
                kind,
            });
        }
    }
}

/// One spec-id reference within a single line, with the byte range of
/// the matched ID inside the line. Returned by [`find_citations_in_line`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineCitation {
    pub id: SpecId,
    /// Byte offsets within the source line.
    pub byte_range: std::ops::Range<usize>,
}

/// Extract every valid `@spec` reference on `line`, with the byte range
/// of each spec ID. The line must contain the `@spec` keyword for any
/// match to count — bare spec-ID-shaped tokens elsewhere on the line
/// are ignored. Empty result for lines without `@spec`.
///
/// This is the line-level building block shared by the source scanner
/// and the LSP server's hover / definition handlers.
#[must_use]
pub fn find_citations_in_line(line: &str) -> Vec<LineCitation> {
    if !AT_SPEC_KEYWORD_RE.is_match(line) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for m in SPEC_ID_IN_TEXT_RE.find_iter(line) {
        let candidate = m.as_str();
        if !candidate.bytes().any(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(id) = SpecId::parse(candidate) {
            out.push(LineCitation {
                id,
                byte_range: m.start()..m.end(),
            });
        }
    }
    out
}

/// Heuristic classification of a path into a [`CitationKind`].
///
/// Markdown / YAML extensions are documentation regardless of where the
/// file lives — a `.md` file under `tests/fixtures/` is still prose,
/// not a test. That lets the doc tree contain `@spec` examples without
/// the source scanner mistaking them for actual citations from tests.
///
/// For code-shaped extensions, paths whose components include `tests`
/// or `test` (or whose filename matches `*_test.go` / `*.test.ts` /
/// `test_*.py` conventions) classify as Test; the rest of the
/// recognised source extensions classify as Code; anything else
/// classifies as Other.
fn classify_path(path: &Path) -> CitationKind {
    let ext = path.extension().and_then(|e| e.to_str());

    // Documentation extensions never count as Test or Code.
    if matches!(ext, Some("md" | "yaml" | "yml")) {
        if path_has_component(path, "specs") && path_has_component(path, "docs") {
            return CitationKind::Spec;
        }
        return CitationKind::Other;
    }

    if path_has_component(path, "tests")
        || path_has_component(path, "test")
        || is_test_filename(path)
    {
        return CitationKind::Test;
    }

    match ext {
        Some(
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "scala" | "rb" | "kt"
            | "swift" | "cs" | "cpp" | "c" | "h" | "hpp",
        ) => CitationKind::Code,
        _ => CitationKind::Other,
    }
}

fn path_has_component(path: &Path, target: &str) -> bool {
    path.components().any(|c| match c {
        std::path::Component::Normal(s) => s == target,
        _ => false,
    })
}

fn is_test_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.contains(".test.")
        || name.contains(".spec.")
        || name.contains("_test.")
        || name.ends_with("_test.go")
        || name.starts_with("test_")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn scan(content: &str, path: &str) -> Vec<SpecCitation> {
        let mut out = Vec::new();
        scan_content(content, Path::new(path), &mut out);
        out
    }

    #[test]
    fn finds_single_citation() {
        let out = scan("// @spec AUTH-001\nfn login() {}\n", "src/auth.rs");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id.as_str(), "AUTH-001");
        assert_eq!(out[0].line, 1);
    }

    #[test]
    fn finds_multiple_citations_on_one_line() {
        let out = scan(
            "// @spec AUTH-001, AUTH-002, AUTH-003\nfn login() {}\n",
            "src/auth.rs",
        );
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].id.as_str(), "AUTH-001");
        assert_eq!(out[1].id.as_str(), "AUTH-002");
        assert_eq!(out[2].id.as_str(), "AUTH-003");
    }

    #[test]
    fn captures_correct_line_numbers() {
        let content = "\
fn one() {}
// @spec AUTH-001
fn two() {}

// @spec AUTH-002
fn three() {}
";
        let out = scan(content, "src/auth.rs");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].line, 2);
        assert_eq!(out[1].line, 5);
    }

    #[test]
    fn ignores_lines_without_at_spec_keyword() {
        // AUTH-001 appears but no `@spec` marker — should not be captured.
        let out = scan("const SPEC_NAMES = ['AUTH-001'];\n", "src/x.ts");
        assert!(out.is_empty());
    }

    #[test]
    fn ignores_at_specification_pseudo_keyword() {
        // `@specification` should not trigger because of the `\b` after `@spec`.
        let out = scan("// @specification of AUTH-001 follows\n", "src/x.ts");
        assert!(out.is_empty());
    }

    #[test]
    fn ignores_no_digit_pseudo_id() {
        // `A-Z` matches the shape regex but has no digit — must be skipped.
        let out = scan("// @spec A-Z\nfn x() {}\n", "src/x.rs");
        assert!(out.is_empty());
    }

    #[test]
    fn ignores_mid_camelcase_hits() {
        // Without `\b…\b` we'd match `GSI-By` inside `GSI-ByReminderDue`.
        let out = scan("// @spec AUTH-001 see also GSI-ByReminderDue\n", "src/x.ts");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id.as_str(), "AUTH-001");
    }

    #[test]
    fn classifies_code_paths() {
        let out = scan("// @spec AUTH-001\n", "src/auth/login.ts");
        assert_eq!(out[0].kind, CitationKind::Code);
    }

    #[test]
    fn classifies_test_paths_via_directory() {
        let out = scan("// @spec AUTH-001\n", "tests/auth/login.ts");
        assert_eq!(out[0].kind, CitationKind::Test);
    }

    #[test]
    fn classifies_test_paths_via_filename_convention() {
        for path in [
            "src/auth/login.test.ts",
            "src/auth/login.spec.ts",
            "internal/auth/login_test.go",
            "auth/test_login.py",
        ] {
            let out = scan("# @spec AUTH-001\n", path);
            assert_eq!(out[0].kind, CitationKind::Test, "path: {path}");
        }
    }

    #[test]
    fn classifies_spec_doc_paths() {
        let out = scan("- @spec AUTH-001\n", "docs/specs/auth-specs.md");
        assert_eq!(out[0].kind, CitationKind::Spec);
    }

    #[test]
    fn markdown_under_tests_directory_is_not_classified_as_test() {
        // Doc fixtures often live under `tests/fixtures/…/foo.md`; a
        // `.md` extension wins over the path-component heuristic so
        // illustrative `@spec` examples don't get treated as real
        // test citations.
        let out = scan(
            "@spec AUTH-001 — illustrative example\n",
            "crates/cli/tests/fixtures/sample.md",
        );
        assert_eq!(out[0].kind, CitationKind::Other);
    }

    #[test]
    fn yaml_files_classify_as_other() {
        let out = scan("# @spec AUTH-001\n", "ci/pipeline.yaml");
        assert_eq!(out[0].kind, CitationKind::Other);
    }

    #[test]
    fn scan_path_walks_directory_respecting_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // `.gitignore` rules only apply inside a git repo; create a
        // minimal `.git` marker so `WalkBuilder` activates them.
        fs::create_dir(root.join(".git")).unwrap();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.rs"), "// @spec A-001\n").unwrap();
        fs::write(root.join("src/b.rs"), "// no marker AUTH-001\n").unwrap();
        fs::write(root.join("ignored.txt"), "// @spec AUTH-002\n").unwrap();
        fs::write(root.join(".gitignore"), "ignored.txt\n").unwrap();

        let mut citations = scan_path(root);
        citations.sort_by(|x, y| x.file.cmp(&y.file));

        // Only src/a.rs should contribute; src/b.rs has no `@spec`, and
        // ignored.txt is filtered out by .gitignore.
        assert_eq!(citations.len(), 1);
        assert!(citations[0].file.ends_with("a.rs"));
        assert_eq!(citations[0].id.as_str(), "A-001");
    }

    #[test]
    fn scan_path_handles_binary_files_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("binary.bin"), [0u8, 1, 2, 0xff]).unwrap();
        fs::write(root.join("good.rs"), "// @spec GOOD-001\n").unwrap();

        let citations = scan_path(root);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id.as_str(), "GOOD-001");
    }

    #[test]
    fn find_citations_in_line_returns_byte_ranges() {
        let line = "// @spec AUTH-001, AUTH-002 trailing comment";
        let cits = find_citations_in_line(line);
        assert_eq!(cits.len(), 2);
        assert_eq!(cits[0].id.as_str(), "AUTH-001");
        assert_eq!(&line[cits[0].byte_range.clone()], "AUTH-001");
        assert_eq!(cits[1].id.as_str(), "AUTH-002");
        assert_eq!(&line[cits[1].byte_range.clone()], "AUTH-002");
    }

    #[test]
    fn find_citations_in_line_ignores_lines_without_at_spec() {
        assert!(find_citations_in_line("const NAMES = ['AUTH-001'];").is_empty());
    }

    #[test]
    fn citation_paths_are_returned_verbatim() {
        let mut out: Vec<SpecCitation> = Vec::new();
        scan_content(
            "// @spec AUTH-001\n",
            Path::new("relative/path/auth.rs"),
            &mut out,
        );
        assert_eq!(out[0].file, PathBuf::from("relative/path/auth.rs"));
    }
}
