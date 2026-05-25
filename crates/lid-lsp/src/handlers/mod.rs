//! LSP request handlers.
//!
//! Each submodule is a pure function from `(LidRepo, document text,
//! cursor position)` to an LSP response value. The `LanguageServer`
//! impl in `crate::server` is a thin async wrapper that fetches the
//! state, calls into one of these, and serialises the result.
//!
//! Keeping the logic pure lets us unit-test every handler against a
//! programmatically-constructed `LidRepo` without any LSP plumbing.

pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod hover;
