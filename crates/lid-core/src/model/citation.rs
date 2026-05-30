//! `@spec` citations — references to spec IDs found in source files.
//!
//! Produced by the scanner in `parse::source`. The coherence checks use
//! these to detect *reverse orphans* (`@spec` references in code with no
//! matching spec line) and *uncovered specs* (spec lines with no `@spec`
//! reference in any test file).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ids::SpecId;

/// Where on disk a `@spec` citation appears, and the role of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CitationKind {
    /// A regular source file under the project's code tree.
    Code,
    /// A test file — used by the coverage check to credit a citation as
    /// satisfying a spec's eval-assertion requirement. The parser uses
    /// a path heuristic (filename contains `test`, or under a `tests/`
    /// directory) to choose this.
    Test,
    /// Inside a spec file under `docs/specs/` — only seen as the
    /// methodology's LID-on-LID inverted pointer, not as ordinary code
    /// coverage.
    Spec,
    /// Anywhere else (docs prose, examples, fixtures).
    Other,
}

impl CitationKind {
    /// Return the canonical lowercase string matching the serde representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Test => "test",
            Self::Spec => "spec",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for CitationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One occurrence of `@spec SPEC-ID` in some file.
///
/// A single line in a source file may produce *multiple* citations when
/// the author writes `@spec A-001, A-002`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecCitation {
    pub id: SpecId,
    /// Path on disk, repo-relative when produced by `LidRepo::discover`.
    pub file: PathBuf,
    /// 1-based line number in the source file.
    pub line: usize,
    pub kind: CitationKind,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn citation_round_trips() {
        let cite = SpecCitation {
            id: SpecId::parse("AUTH-001").unwrap(),
            file: PathBuf::from("src/auth/login.ts"),
            line: 12,
            kind: CitationKind::Code,
        };
        let json = serde_json::to_string(&cite).unwrap();
        let back: SpecCitation = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cite);
    }

    #[test]
    fn citation_kind_as_str_matches_serde() {
        for (k, _) in [
            (CitationKind::Code, "\"code\""),
            (CitationKind::Test, "\"test\""),
            (CitationKind::Spec, "\"spec\""),
            (CitationKind::Other, "\"other\""),
        ] {
            let serde_form = serde_json::to_string(&k).unwrap();
            let serde_str = serde_form.trim_matches('"');
            assert_eq!(k.as_str(), serde_str);
            assert_eq!(k.to_string(), serde_str);
        }
    }

    #[test]
    fn citation_kind_serializes_lowercase() {
        for (k, expected) in [
            (CitationKind::Code, "\"code\""),
            (CitationKind::Test, "\"test\""),
            (CitationKind::Spec, "\"spec\""),
            (CitationKind::Other, "\"other\""),
        ] {
            assert_eq!(serde_json::to_string(&k).unwrap(), expected);
        }
    }
}
