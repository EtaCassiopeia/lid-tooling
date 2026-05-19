//! Typed data model for LID artifacts.
//!
//! Each submodule owns one family of types; downstream code should depend
//! on the smallest module that fits rather than glob-importing from here.

pub mod arrow;
pub mod citation;
pub mod ids;
pub mod lld;
pub mod spec;

pub use arrow::{ArrowIndex, Edge, EdgeKind, Segment, Status, Unmapped, UnmappedDocs};
pub use citation::{CitationKind, SpecCitation};
pub use ids::{GitSha, SegmentId, SpecId};
pub use lld::{DecisionRow, LldDoc};
pub use spec::{SpecFile, SpecLine, SpecStatus};
