//! Typed data model for LID artifacts.
//!
//! Each submodule owns one family of types; downstream code should depend
//! on the smallest module that fits rather than glob-importing from here.

pub mod ids;

pub use ids::{GitSha, SpecId};
