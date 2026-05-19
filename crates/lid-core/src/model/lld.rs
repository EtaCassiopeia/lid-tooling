//! Typed model for Low-Level Design documents (`docs/llds/*.md`).
//!
//! The model captures only the structured pieces the coherence checks
//! consult — primarily the Decisions & Alternatives table — and leaves
//! the rest as opaque prose. The parser in `parse::markdown` populates
//! these types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A row in an LLD's `Decisions & Alternatives` table.
///
/// Marked `inferred` when the row text contains an `[inferred]` tag, a
/// brownfield-import convention indicating the entry was reconstructed
/// from existing code rather than authored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRow {
    pub decision: String,
    pub chosen: String,
    pub alternatives: String,
    pub rationale: String,
    #[serde(default)]
    pub inferred: bool,
}

/// A parsed LLD document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LldDoc {
    /// Path on disk, repo-relative when produced by `LidRepo::discover`.
    pub path: PathBuf,
    /// Rows in the `Decisions & Alternatives` table, in source order.
    /// Empty when the section is absent.
    #[serde(default)]
    pub decisions: Vec<DecisionRow>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn decision_row_defaults_inferred_to_false() {
        let json = r#"{
            "decision": "Storage layer",
            "chosen": "PostgreSQL",
            "alternatives": "SQLite, DynamoDB",
            "rationale": "Existing infra"
        }"#;
        let row: DecisionRow = serde_json::from_str(json).unwrap();
        assert!(!row.inferred);
    }

    #[test]
    fn lld_doc_round_trips() {
        let lld = LldDoc {
            path: PathBuf::from("docs/llds/auth.md"),
            decisions: vec![DecisionRow {
                decision: "Session store".to_owned(),
                chosen: "Redis".to_owned(),
                alternatives: "Memcached, DB session table".to_owned(),
                rationale: "Latency and TTL semantics".to_owned(),
                inferred: true,
            }],
        };
        let json = serde_json::to_string(&lld).unwrap();
        let back: LldDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back, lld);
    }
}
