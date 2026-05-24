//! The LSP server type.
//!
//! `LidServer` implements [`tower_lsp::LanguageServer`]. State that
//! the handlers consult (today: the [`DocStore`]) lives on the server
//! itself so the impl methods stay short and the unit-testable logic
//! sits in `lid_core`.

use lid_core::DocStore;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp::{Client, LanguageServer, jsonrpc::Result};

#[derive(Debug)]
pub struct LidServer {
    client: Client,
    store: DocStore,
}

impl LidServer {
    /// Constructor used by `LspService::new`.
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            store: DocStore::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LidServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        // Capabilities grow as handlers land. Today: full-text sync
        // (the simplest mode — server gets the whole buffer on every
        // change) and nothing else.
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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let td = params.text_document;
        let uri = td.uri.to_string();
        tracing::debug!(uri = %uri, version = td.version, "did_open");
        let _ = self.store.upsert(uri, td.text, td.version);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;
        // We registered FULL sync, so the first (and only) change
        // entry carries the entire new buffer.
        let Some(change) = params.content_changes.into_iter().next() else {
            tracing::warn!(uri = %uri, "did_change with no content");
            return;
        };
        tracing::debug!(uri = %uri, version, "did_change");
        let _ = self.store.upsert(uri, change.text, version);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        tracing::debug!(uri = %uri, "did_close");
        let _ = self.store.remove(&uri);
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("shutdown requested");
        Ok(())
    }
}
