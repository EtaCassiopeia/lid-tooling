//! `lid-lsp` — Language Server Protocol implementation for LID.
//!
//! Today the binary speaks LSP over stdio and responds to the initialize
//! handshake. Real handlers (hover, definition, diagnostics, rename)
//! land in subsequent commits. The structure exists already so each
//! handler is one focused change.

mod handlers;
mod server;

use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;

use crate::server::LidServer;

#[tokio::main]
async fn main() {
    // Logging routes to stderr so it doesn't pollute LSP's stdio
    // channel. Set `RUST_LOG=debug` or similar to bump verbosity.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "lid-lsp starting");

    let (service, socket) = LspService::new(LidServer::new);
    Server::new(stdin(), stdout(), socket).serve(service).await;
}
