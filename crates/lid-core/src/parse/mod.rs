//! Parsers for LID artifact formats.
//!
//! Each submodule is a thin layer over a third-party parser plus light
//! validation; the typed output lives in `crate::model`. Parsers never
//! consult the filesystem beyond the single file they are given — file
//! discovery is the responsibility of `crate::repo`.

pub mod markdown;
pub mod yaml;
