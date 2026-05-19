//! Typed data model for LID artifacts.
//!
//! Each submodule owns one family of types; downstream code should depend
//! on the smallest module that fits rather than glob-importing from here.

pub mod arrow;
pub mod ids;

pub use arrow::{ArrowIndex, Edge, EdgeKind, Segment, Status, Unmapped, UnmappedDocs};
pub use ids::{GitSha, SegmentId, SpecId};
