//! Typed data model for the arrow overlay (`docs/arrows/index.yaml`).
//!
//! These are pure data types — deserialization lives in `parse::yaml` and
//! checks live in `checks::*`. The model intentionally mirrors the YAML
//! schema rather than introducing derived state, so a round-trip
//! `parse -> serialize` produces equivalent YAML.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::ids::{GitSha, SegmentId};

/// Lifecycle status of an arrow segment, exactly as encoded in `index.yaml`.
///
/// The order of variants follows the methodology's normal progression
/// (`UNMAPPED → MAPPED → AUDITED → OK`) with the recoverable detours
/// (`PARTIAL`, `BROKEN`, `STALE`) and terminal states (`OBSOLETE`,
/// `MERGED`) after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Unmapped,
    Mapped,
    Audited,
    Ok,
    Partial,
    Broken,
    Stale,
    Obsolete,
    Merged,
}

/// One arrow segment — the value under `arrows.{segment-id}` in `index.yaml`.
///
/// All optional fields default to `None` / empty so that minimal YAML
/// entries (just `status: UNMAPPED` plus `detail: …`) deserialize cleanly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub status: Status,

    #[serde(default)]
    pub sampled: Option<NaiveDate>,

    #[serde(default)]
    pub audited: Option<NaiveDate>,

    #[serde(default)]
    pub audited_sha: Option<GitSha>,

    #[serde(default)]
    pub blocks: Vec<SegmentId>,

    #[serde(default, rename = "blockedBy")]
    pub blocked_by: Vec<SegmentId>,

    /// Relative filename of the segment's detail doc, e.g.
    /// `linked-intent-dev.md` (relative to `docs/arrows/`).
    pub detail: PathBuf,

    #[serde(default)]
    pub next: Option<String>,

    #[serde(default)]
    pub drift: Option<String>,

    /// Target segment when `status == Status::Merged`. Required only in
    /// that state; absent otherwise.
    #[serde(default)]
    pub merged_into: Option<SegmentId>,
}

/// Orphans — files known to exist under `docs/llds/` or `docs/specs/` but
/// not yet attached to any arrow segment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmappedDocs {
    #[serde(default)]
    pub llds: Vec<PathBuf>,

    #[serde(default)]
    pub specs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unmapped {
    #[serde(default)]
    pub docs: UnmappedDocs,
}

/// Top-level `docs/arrows/index.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowIndex {
    pub schema_version: u32,

    #[serde(default)]
    pub last_updated: Option<NaiveDate>,

    /// Free-form grouping of segments under named clusters (e.g. `core`,
    /// `experimental`). `BTreeMap` keeps cluster order stable across
    /// serialize round-trips.
    #[serde(default)]
    pub taxonomy: BTreeMap<String, Vec<SegmentId>>,

    #[serde(default)]
    pub arrows: BTreeMap<SegmentId, Segment>,

    #[serde(default)]
    pub unmapped: Unmapped,
}

/// Kind of relationship represented by an [`Edge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// `from` blocks `to` (i.e. `to` is `blockedBy: [from]`).
    Blocks,
    /// `from.merged_into = to` (only when `from.status == Merged`).
    MergedInto,
}

/// Directed relationship between two segments. Computed view — not stored
/// in the YAML directly; produced from `Segment::blocks` and `merged_into`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edge {
    pub from: SegmentId,
    pub to: SegmentId,
    pub kind: EdgeKind,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn seg(id: &str) -> SegmentId {
        SegmentId::parse(id).unwrap()
    }

    #[test]
    fn segment_with_only_required_fields_constructs() {
        let s = Segment {
            status: Status::Unmapped,
            sampled: None,
            audited: None,
            audited_sha: None,
            blocks: vec![],
            blocked_by: vec![],
            detail: PathBuf::from("auth.md"),
            next: None,
            drift: None,
            merged_into: None,
        };
        assert_eq!(s.status, Status::Unmapped);
    }

    #[test]
    fn arrow_index_serializes_to_yaml_shape() {
        // Construct a minimal ArrowIndex; serialize as JSON (cheap; YAML
        // uses the same serde data model) and assert the field names
        // match the upstream YAML schema.
        let mut arrows = BTreeMap::new();
        arrows.insert(
            seg("linked-intent-dev"),
            Segment {
                status: Status::Mapped,
                sampled: Some(NaiveDate::from_ymd_opt(2026, 5, 5).unwrap()),
                audited: None,
                audited_sha: None,
                blocks: vec![seg("lid-coach")],
                blocked_by: vec![],
                detail: PathBuf::from("linked-intent-dev.md"),
                next: None,
                drift: None,
                merged_into: None,
            },
        );
        let idx = ArrowIndex {
            schema_version: 1,
            last_updated: None,
            taxonomy: BTreeMap::new(),
            arrows,
            unmapped: Unmapped::default(),
        };
        let json = serde_json::to_value(&idx).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(
            json["arrows"]["linked-intent-dev"]["status"],
            serde_json::Value::String("MAPPED".into())
        );
        assert_eq!(
            json["arrows"]["linked-intent-dev"]["blockedBy"],
            serde_json::json!([])
        );
        assert_eq!(
            json["arrows"]["linked-intent-dev"]["blocks"],
            serde_json::json!(["lid-coach"])
        );
    }

    #[test]
    fn arrow_index_round_trips_through_json() {
        let mut arrows = BTreeMap::new();
        arrows.insert(
            seg("auth"),
            Segment {
                status: Status::Ok,
                sampled: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
                audited: Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
                audited_sha: Some(GitSha::parse("a1b2c3d").unwrap()),
                blocks: vec![],
                blocked_by: vec![seg("session")],
                detail: PathBuf::from("auth.md"),
                next: Some("write tests".into()),
                drift: None,
                merged_into: None,
            },
        );
        let idx = ArrowIndex {
            schema_version: 1,
            last_updated: Some(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap()),
            taxonomy: BTreeMap::from([("core".to_owned(), vec![seg("auth")])]),
            arrows,
            unmapped: Unmapped::default(),
        };

        let json = serde_json::to_string(&idx).unwrap();
        let back: ArrowIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(back, idx);
    }

    #[test]
    fn status_round_trips_each_variant() {
        for v in [
            Status::Unmapped,
            Status::Mapped,
            Status::Audited,
            Status::Ok,
            Status::Partial,
            Status::Broken,
            Status::Stale,
            Status::Obsolete,
            Status::Merged,
        ] {
            let s = serde_json::to_string(&v).unwrap();
            let back: Status = serde_json::from_str(&s).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn status_serializes_as_screaming_snake() {
        assert_eq!(serde_json::to_string(&Status::Ok).unwrap(), "\"OK\"");
        assert_eq!(
            serde_json::to_string(&Status::Mapped).unwrap(),
            "\"MAPPED\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Merged).unwrap(),
            "\"MERGED\""
        );
    }

    #[test]
    fn edge_kind_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&EdgeKind::Blocks).unwrap(),
            "\"blocks\""
        );
        assert_eq!(
            serde_json::to_string(&EdgeKind::MergedInto).unwrap(),
            "\"merged_into\""
        );
    }
}
