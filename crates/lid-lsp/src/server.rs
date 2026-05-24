//! The LSP server type.
//!
//! `LidServer` implements [`tower_lsp::LanguageServer`]; the concrete
//! handlers grow one commit at a time as features land. Today the
//! server only completes the initialize/shutdown handshake — enough
//! for an editor to connect and confirm the binary speaks LSP.

use tower_lsp::lsp_types::{
    InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp::{Client, LanguageServer, jsonrpc::Result};

#[derive(Debug)]
pub struct LidServer {
    client: Client,
}

impl LidServer {
    /// Constructor used by `LspService::new`. Held private to the
    /// crate; external callers don't construct servers directly.
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LidServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        // Capabilities are intentionally minimal at this stage. Each
        // future handler commit adds its capability flag here.
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            ..ServerCapabilities::default()
        };

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            capabilities,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("lid-lsp {} ready", env!("CARGO_PKG_VERSION")),
            )
            .await;
        tracing::info!("client connected, server is ready");
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("shutdown requested");
        Ok(())
    }
}
