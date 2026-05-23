//! Coherence checks.
//!
//! Each check is a small unit that reads from a `&LidRepo` and emits a
//! `Vec<Finding>`. Checks never modify the repo, never touch the
//! filesystem outside what `LidRepo` already loaded, and never short-
//! circuit on the first finding — a single run should surface every
//! issue a check can detect, so the CLI's report is complete.
//!
//! Concrete checks live in submodules; this module owns the trait and
//! the common report types.

pub mod orphans;
pub mod references;
pub mod reverse_orphan;
pub mod schema;
pub mod spec_id_format;

pub use orphans::OrphanCheck;
pub use references::ReferenceCoherenceCheck;
pub use reverse_orphan::ReverseOrphanCheck;
pub use schema::SchemaCheck;
pub use spec_id_format::SpecIdFormatCheck;

/// The default ordered list of checks to run in a `lidc check` pass.
///
/// Adapters (CLI, LSP, MCP) should call this rather than instantiating
/// concrete checks themselves so that adding a new check is a one-line
/// change to the registry instead of a sweep across every consumer.
#[must_use]
pub fn default_checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(SchemaCheck),
        Box::new(ReferenceCoherenceCheck),
        Box::new(OrphanCheck),
        Box::new(ReverseOrphanCheck),
        Box::new(SpecIdFormatCheck),
    ]
}

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::LidRepo;
use crate::model::SpecId;

/// Stable identifier for a check.
///
/// Used by `Finding.check` so CLI / LSP consumers can filter by check,
/// and by the `--only` flag in the eventual CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum CheckId {
    Schema,
    ReferenceCoherence,
    Coverage,
    Orphan,
    ReverseOrphan,
    SpecIdFormat,
    SpecStatusCounts,
    LldDecisions,
    Dag,
}

impl CheckId {
    /// The kebab-case string form used by JSON output, the `--only` flag,
    /// and human-readable rendering. Kept in lockstep with the serde
    /// representation by a unit test below.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::ReferenceCoherence => "reference-coherence",
            Self::Coverage => "coverage",
            Self::Orphan => "orphan",
            Self::ReverseOrphan => "reverse-orphan",
            Self::SpecIdFormat => "spec-id-format",
            Self::SpecStatusCounts => "spec-status-counts",
            Self::LldDecisions => "lld-decisions",
            Self::Dag => "dag",
        }
    }
}

impl std::fmt::Display for CheckId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How serious a finding is.
///
/// `Error` and `Warning` are surfaced by default; `Info` is hidden
/// unless the user opts in (e.g. `lidc check --info`). The CLI's
/// `--fail-on` flag chooses where exit-code 1 begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Broad grouping that mirrors the audit-checklist's sections; used for
/// human-friendly report headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Schema,
    References,
    Coverage,
    Staleness,
    Dag,
    Lint,
    Orphans,
}

/// A location in the repository — typically the file and 1-based line
/// the finding refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// One observation produced by a check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub check: CheckId,
    pub severity: Severity,
    pub category: Category,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<SpecId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

/// A coherence check.
///
/// Checks are stateless; the implementation receives the repository and
/// returns every finding it can produce. Order within the returned
/// `Vec` is not significant — the renderer sorts findings before
/// presenting them.
pub trait Check: Send + Sync {
    /// Stable identifier for this check (also stamped onto every
    /// finding it produces).
    fn id(&self) -> CheckId;

    /// Run the check against `repo` and return all findings produced.
    fn run(&self, repo: &LidRepo) -> Vec<Finding>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn finding_serializes_with_omitted_optional_fields() {
        let f = Finding {
            check: CheckId::Schema,
            severity: Severity::Error,
            category: Category::Schema,
            message: "boom".to_owned(),
            location: None,
            spec: None,
            remediation: None,
        };
        let json = serde_json::to_string(&f).unwrap();
        // Optional fields are skipped, keeping JSON compact.
        assert!(!json.contains("location"));
        assert!(!json.contains("\"spec\""));
        assert!(!json.contains("remediation"));
    }

    #[test]
    fn finding_round_trips_with_location_and_spec() {
        let f = Finding {
            check: CheckId::ReverseOrphan,
            severity: Severity::Warning,
            category: Category::Orphans,
            message: "reverse orphan".to_owned(),
            location: Some(Location {
                path: PathBuf::from("src/auth.rs"),
                line: Some(42),
            }),
            spec: Some(SpecId::parse("AUTH-001").unwrap()),
            remediation: Some("create the spec or delete the annotation".to_owned()),
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(back, f);
    }

    #[test]
    fn severity_orders_info_lt_warning_lt_error() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn severity_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Severity::Warning).unwrap(),
            "\"warning\""
        );
    }

    #[test]
    fn check_id_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&CheckId::Schema).unwrap(),
            "\"schema\""
        );
        assert_eq!(
            serde_json::to_string(&CheckId::ReferenceCoherence).unwrap(),
            "\"reference-coherence\""
        );
        assert_eq!(
            serde_json::to_string(&CheckId::ReverseOrphan).unwrap(),
            "\"reverse-orphan\""
        );
    }

    #[test]
    fn check_id_as_str_matches_serde_representation() {
        for id in [
            CheckId::Schema,
            CheckId::ReferenceCoherence,
            CheckId::Coverage,
            CheckId::Orphan,
            CheckId::ReverseOrphan,
            CheckId::SpecIdFormat,
            CheckId::SpecStatusCounts,
            CheckId::LldDecisions,
            CheckId::Dag,
        ] {
            let serde_form = serde_json::to_string(&id).unwrap();
            let serde_form = serde_form.trim_matches('"');
            assert_eq!(
                id.as_str(),
                serde_form,
                "as_str() must match serde rename for {id:?}"
            );
            assert_eq!(id.to_string(), serde_form);
        }
    }
}
