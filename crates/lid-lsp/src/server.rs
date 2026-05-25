//! The LSP server type.
//!
//! `LidServer` implements [`tower_lsp::LanguageServer`]. State that
//! the handlers consult (today: the [`DocStore`] and the lazily-loaded
//! [`LidRepo`]) lives on the server itself so the impl methods stay
//! short and the unit-testable logic sits in `crate::handlers`.

use lid_core::{DocStore, LidRepo};
use tokio::sync::OnceCell;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, MessageType, OneOf, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer, jsonrpc::Result};

use crate::handlers;

#[derive(Debug)]
pub struct LidServer {
    client: Client,
    store: DocStore,
    /// The discovered LID repository for this session. Initialised on
    /// first request that needs it — typically the first hover.
    /// Outer cell guarantees init-once; inner `Option` records "we
    /// tried, no repo found" so the failure isn't retried.
    repo: OnceCell<Option<LidRepo>>,
}

impl LidServer {
    /// Constructor used by `LspService::new`.
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            store: DocStore::new(),
            repo: OnceCell::new(),
        }
    }

    /// Compute and publish diagnostics for `uri` using the buffer `text`
    /// as the source of truth (rather than what's on disk). Silently
    /// no-ops when no LID repository is reachable from the URI.
    async fn publish_diagnostics_for(&self, uri: Url, version: Option<i32>, text: &str) {
        let Some(repo) = self.ensure_repo(&uri).await else {
            return;
        };
        let diagnostics = handlers::diagnostics::diagnostics_for_buffer(repo, text);
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    /// Discover (or return the cached) `LidRepo` using the given URI
    /// as a starting hint for the upward walk. Returns `None` when
    /// the URI can't be converted to a path or no LID repo is reachable.
    async fn ensure_repo(&self, hint: &Url) -> Option<&LidRepo> {
        let cached = self
            .repo
            .get_or_init(|| async {
                let Ok(path) = hint.to_file_path() else {
                    tracing::debug!(uri = %hint, "uri is not a file path; skipping repo discovery");
                    return None;
                };
                match LidRepo::discover(&path) {
                    Ok(repo) => {
                        tracing::info!(root = ?repo.root, "LID repo discovered");
                        Some(repo)
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "no LID repo at or above {}", path.display());
                        None
                    }
                }
            })
            .await;
        cached.as_ref()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LidServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
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
        let uri = td.uri.clone();
        let version = td.version;
        let text = td.text;
        tracing::debug!(uri = %uri, version, "did_open");
        let _ = self
            .store
            .upsert(uri.as_str().to_owned(), text.clone(), version);
        self.publish_diagnostics_for(uri, Some(version), &text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        let Some(change) = params.content_changes.into_iter().next() else {
            tracing::warn!(uri = %uri, "did_change with no content");
            return;
        };
        let text = change.text;
        tracing::debug!(uri = %uri, version, "did_change");
        let _ = self
            .store
            .upsert(uri.as_str().to_owned(), text.clone(), version);
        self.publish_diagnostics_for(uri, Some(version), &text)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        tracing::debug!(uri = %uri, "did_close");
        let _ = self.store.remove(uri.as_str());
        // Clear any diagnostics we'd previously published for this URI.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.store.get(uri.as_str()) else {
            return Ok(None);
        };
        let Some(repo) = self.ensure_repo(&uri).await else {
            return Ok(None);
        };
        Ok(handlers::hover::hover_at_position(
            repo, &doc.text, position,
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.store.get(uri.as_str()) else {
            return Ok(None);
        };
        let Some(repo) = self.ensure_repo(&uri).await else {
            return Ok(None);
        };
        Ok(handlers::definition::definition_at_position(
            repo, &doc.text, position,
        ))
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("shutdown requested");
        Ok(())
    }
}
