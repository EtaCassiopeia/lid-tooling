//! Domain error type for `lid-core`.
//!
//! Adapters (`lid-cli`, `lid-lsp`) typically wrap this in `anyhow::Error`
//! for ergonomic propagation at their boundary; the library itself never
//! reaches for `anyhow`.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Top-level error returned by `lid-core` operations.
///
/// Marked `#[non_exhaustive]` so new variants (one per parser/check group)
/// can be added without a breaking change.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LidError {
    /// Filesystem error encountered while reading a LID artifact.
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// YAML deserialization error.
    #[error("YAML parse error in {path}: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: Box<serde_yaml_ng::Error>,
    },

    /// Markdown parse error with file + line context.
    ///
    /// Carries an opaque source so that any validation error
    /// (`InvalidSpecId`, future variants) can be wrapped with a locatable
    /// position without forcing every markdown caller to know which
    /// concrete variant is underneath.
    #[error("{path}:{line}: {source}")]
    Markdown {
        path: PathBuf,
        line: usize,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// A `SpecId` failed validation against the `^[A-Z][A-Z0-9-]+$` shape.
    #[error("invalid spec id {value:?}: {reason}")]
    InvalidSpecId { value: String, reason: &'static str },

    /// A `GitSha` failed validation against the 7..=40 hex-digit shape.
    #[error("invalid git sha {value:?}: {reason}")]
    InvalidGitSha { value: String, reason: &'static str },

    /// A `SegmentId` failed validation against the kebab-case shape.
    #[error("invalid segment id {value:?}: {reason}")]
    InvalidSegmentId { value: String, reason: &'static str },

    /// No LID repository (no `docs/arrows/index.yaml`) was found at or above
    /// the starting path during discovery.
    #[error("no LID repository found at or above {start}")]
    NotALidRepo { start: PathBuf },

    /// The project's `schema_version` is not in `SUPPORTED_SCHEMA_VERSIONS`.
    ///
    /// Returned by `LidRepo::discover` before any artifact loading so that
    /// callers never receive a partially-loaded repo from an unknown layout.
    #[error("schema_version {found} is not supported (supported: {supported:?})")]
    UnsupportedSchemaVersion {
        found: u32,
        supported: &'static [u32],
    },
}

/// Convenience alias for results carrying a [`LidError`].
pub type Result<T, E = LidError> = core::result::Result<T, E>;
