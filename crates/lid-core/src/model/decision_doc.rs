//! Typed model for standalone decision documents.
//!
//! LID 1.2.0 introduces decision docs as first-class artifacts separate from
//! the `## Decisions & Alternatives` table embedded in design docs. They live
//! in two locations:
//!
//! * `docs/decisions/` — project-level decisions that span multiple nodes
//! * `docs/intent/<node>/decisions/` — decisions scoped to a single node
//!
//! "Presence is acceptance, no EARS attached." A decision doc records a choice
//! that was made; it carries no spec lines or status tracking.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where a decision document is scoped within the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DecisionScope {
    /// Project-level decision (`docs/decisions/`); applies across nodes.
    Project,
    /// Per-node decision (`docs/intent/<node>/decisions/`); the `segment`
    /// field is the folder name of the owning node (e.g. `"auth"`).
    Node { segment: String },
}

/// A parsed standalone decision document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionDoc {
    /// Absolute path on disk (canonicalised when produced by `LidRepo::discover`).
    pub path: PathBuf,
    /// Where in the project this decision lives.
    pub scope: DecisionScope,
    /// First H1 heading in the file. Empty string when no H1 is present.
    pub title: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn decision_scope_project_serializes_correctly() {
        let scope = DecisionScope::Project;
        let json = serde_json::to_string(&scope).unwrap();
        assert!(json.contains("\"kind\":\"project\""), "got {json}");
    }

    #[test]
    fn decision_scope_node_serializes_correctly() {
        let scope = DecisionScope::Node {
            segment: "auth".to_owned(),
        };
        let json = serde_json::to_string(&scope).unwrap();
        assert!(json.contains("\"kind\":\"node\""), "got {json}");
        assert!(json.contains("\"segment\":\"auth\""), "got {json}");
    }

    #[test]
    fn decision_doc_round_trips() {
        let doc = DecisionDoc {
            path: PathBuf::from("docs/decisions/session-storage.md"),
            scope: DecisionScope::Project,
            title: "Session Storage Strategy".to_owned(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back: DecisionDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn decision_doc_node_scope_round_trips() {
        let doc = DecisionDoc {
            path: PathBuf::from("docs/intent/auth/decisions/token-format.md"),
            scope: DecisionScope::Node {
                segment: "auth".to_owned(),
            },
            title: "Token Format".to_owned(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back: DecisionDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back, doc);
    }
}
