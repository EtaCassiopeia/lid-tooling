//! Typed model for EARS spec files (`docs/specs/*.md`).
//!
//! A spec file is a markdown document whose body lists requirement bullets
//! in the form:
//!
//! ```text
//! - [x] **AUTH-UI-001**: text of the requirement.
//! - [ ] **AUTH-UI-002**: another requirement.
//! - [D] **AUTH-UI-003**: deferred requirement.
//! ```
//!
//! Optionally, the file header carries pointers to the implementing
//! artifacts (LID-on-LID inverted reference) and the LLD that scopes the
//! spec.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ids::SpecId;

/// Status marker on a spec line.
///
/// Maps to the `[x]` / `[ ]` / `[D]` checkbox the methodology uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecStatus {
    /// `[x]` — implemented.
    Implemented,
    /// `[ ]` — active gap.
    Open,
    /// `[D]` — deferred (intentionally not implemented yet).
    Deferred,
}

impl SpecStatus {
    /// Return the canonical lowercase string matching the serde representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Open => "open",
            Self::Deferred => "deferred",
        }
    }
}

impl std::fmt::Display for SpecStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One requirement line inside a spec file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecLine {
    pub id: SpecId,
    pub status: SpecStatus,
    /// The requirement text as authored (after the `**ID**:` prefix).
    pub text: String,
    /// 1-based line number in the source file.
    pub line: usize,
}

/// A parsed spec file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecFile {
    /// Path on disk, repo-relative when produced by `LidRepo::discover`.
    pub path: PathBuf,
    /// Spec lines, in source order.
    pub specs: Vec<SpecLine>,
    /// Files listed under `**Implementing artifacts**:` in the header.
    /// Empty when no such header is present (the common case).
    #[serde(default)]
    pub implementing_artifacts: Vec<PathBuf>,
    /// The LLD this spec file scopes, parsed from a `**LLD**: …` header
    /// line when present.
    #[serde(default)]
    pub lld: Option<PathBuf>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn spec_status_as_str_matches_serde() {
        for v in [
            SpecStatus::Implemented,
            SpecStatus::Open,
            SpecStatus::Deferred,
        ] {
            let serde_form = serde_json::to_string(&v).unwrap();
            let serde_str = serde_form.trim_matches('"');
            assert_eq!(v.as_str(), serde_str);
            assert_eq!(v.to_string(), serde_str);
        }
    }

    #[test]
    fn spec_status_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&SpecStatus::Implemented).unwrap(),
            "\"implemented\""
        );
        assert_eq!(
            serde_json::to_string(&SpecStatus::Open).unwrap(),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&SpecStatus::Deferred).unwrap(),
            "\"deferred\""
        );
    }

    #[test]
    fn spec_line_round_trips() {
        let line = SpecLine {
            id: SpecId::parse("AUTH-001").unwrap(),
            status: SpecStatus::Implemented,
            text: "When the user logs in, the system SHALL …".to_owned(),
            line: 42,
        };
        let json = serde_json::to_string(&line).unwrap();
        let back: SpecLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back, line);
    }
}
