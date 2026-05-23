//! Renderers that turn a list of `Finding`s into something the user
//! can read. Each renderer is pure — it takes a slice of findings and
//! returns a `String` — so `main` decides where to write and tests
//! don't need to capture stdout.

pub mod json;
pub mod markdown;

use std::io::IsTerminal;

/// Knobs that affect rendering. Today there's only one (color); kept
/// as a struct so future flags (verbosity, unicode-vs-ascii markers)
/// land additively.
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub color: bool,
}

impl RenderOptions {
    /// Detect colour support from stdout's TTY status. Use this when the
    /// destination is the user's terminal; explicit choices (--json,
    /// piped output) should construct `RenderOptions` directly.
    #[must_use]
    pub fn from_stdout() -> Self {
        Self {
            color: std::io::stdout().is_terminal(),
        }
    }
}
