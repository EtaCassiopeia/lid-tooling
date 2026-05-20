//! `lid-core` — engine for LID (Linked-Intent Development) tooling.
//!
//! Parses LID artifacts (`docs/arrows/index.yaml`, EARS spec files, LLDs,
//! arrow docs) and `@spec` annotations in source code, then runs the
//! deterministic coherence checks the methodology specifies.
//!
//! See <https://github.com/jszmajda/lid> for the methodology this crate
//! supports.

pub mod error;
pub mod model;
pub mod parse;

pub use error::{LidError, Result};
pub use model::{ArrowIndex, GitSha, Segment, SegmentId, SpecId, Status};
