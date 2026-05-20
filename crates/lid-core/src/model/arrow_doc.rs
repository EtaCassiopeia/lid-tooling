//! Typed model for per-segment arrow detail docs (`docs/arrows/{segment}.md`).
//!
//! Each arrow segment in `index.yaml` points to a markdown detail doc via
//! its `detail:` field. The doc's `## References` section is the
//! authoritative inventory of files that *belong* to the segment — the
//! audit-checklist's "reference coherence" check walks each path here and
//! asserts the target exists.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Bullets under `## References / ### {Kind}` in an arrow doc, grouped by
/// the H3 subsection.
///
/// Bullets are stored as raw strings rather than `PathBuf` because the
/// upstream convention permits trailing annotations (e.g. `path.md §Section`
/// for HLD entries). Path resolution happens at check time, where each
/// consumer can decide how strict to be.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowReferences {
    #[serde(default)]
    pub hld: Vec<String>,
    #[serde(default)]
    pub lld: Vec<String>,
    #[serde(default)]
    pub ears: Vec<String>,
    #[serde(default)]
    pub tests: Vec<String>,
    #[serde(default)]
    pub code: Vec<String>,
}

/// A parsed arrow detail doc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowDoc {
    /// Path on disk, repo-relative when produced by `LidRepo::discover`.
    pub path: PathBuf,
    pub references: ArrowReferences,
}
