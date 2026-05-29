//! The LSP server type.
//!
//! `LidServer` implements [`tower_lsp::LanguageServer`]. State that
//! the handlers consult (the [`DocStore`] and the live-reloadable
//! [`LidRepo`]) lives on the server itself so the impl methods stay
//! short and the unit-testable logic sits in `crate::handlers`.
//!
//! ## Live reload
//!
//! The repo is initially discovered on the first request, then kept in
//! an `Arc` inside an `RwLock` so it can be atomically swapped on
//! reload without blocking in-flight handlers. Two events trigger a
//! reload:
//!
//! * **`didSave`** — the editor tells us it flushed a buffer to disk;
//!   we re-read the whole repo so that spec-file and source-file
//!   changes both land immediately.
//! * **`workspace/didChangeWatchedFiles`** — the LSP client notifies
//!   us of on-disk changes made outside the editor (e.g. an AI agent
//!   writing spec files or a `git checkout`). We register watchers in
//!   `initialized` for `**/*.md`, `**/index.yaml`, and common source
//!   extensions.
//!
//! Reload errors (e.g. a file that is half-written while the user is
//! still typing) are swallowed — the previous good repo is kept and a
//! warning is logged. This makes the server resilient to transient
//! filesystem states without surfacing spurious error messages.

use std::path::PathBuf;
use std::sync::Arc;

use lid_core::{DocStore, LidRepo};
use tokio::sync::{OnceCell, RwLock};
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidChangeWatchedFilesRegistrationOptions,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    FileSystemWatcher, GlobPattern, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    Location, MessageType, OneOf, PrepareRenameResponse, ReferenceParams, Registration,
    RenameOptions, RenameParams, ServerCapabilities, ServerInfo, SymbolInformation,
    TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    WorkDoneProgressOptions, WorkspaceEdit, WorkspaceSymbolParams,
};
use tower_lsp::{Client, LanguageServer, jsonrpc::Result};

use crate::handlers;

#[derive(Debug)]
pub struct LidServer {
    client: Client,
    store: DocStore,
    /// Current LID repository snapshot. Wrapped in `Arc` so handlers
    /// can clone a cheap reference without holding the lock across I/O.
    repo: RwLock<Option<Arc<LidRepo>>>,
    /// Path used as the starting hint for repo discovery, cached on
    /// first successful load so reloads know where to look.
    repo_root: OnceCell<PathBuf>,
}

impl LidServer {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            store: DocStore::new(),
            repo: RwLock::new(None),
            repo_root: OnceCell::new(),
        }
    }

    // ── Repo access ────────────────────────────────────────────────────────────

    /// Return the current repo snapshot (cheap `Arc` clone). If no repo
    /// has been discovered yet, attempt discovery using `hint` as the
    /// starting path. Returns `None` when the URI is not a file path or
    /// no LID repo exists above it.
    async fn get_repo(&self, hint: &Url) -> Option<Arc<LidRepo>> {
        // Fast path: already loaded.
        {
            let guard = self.repo.read().await;
            if guard.is_some() {
                return guard.clone();
            }
        }

        // Slow path: first discovery.
        let Ok(path) = hint.to_file_path() else {
            tracing::debug!(uri = %hint, "uri is not a file path; skipping repo discovery");
            return None;
        };
        self.discover_and_store(path).await
    }

    /// Run `LidRepo::discover` on a blocking thread, store the result,
    /// and return it. If discovery fails the error is logged and `None`
    /// is returned without touching the existing cached repo.
    async fn discover_and_store(&self, hint: PathBuf) -> Option<Arc<LidRepo>> {
        let result = tokio::task::spawn_blocking(move || LidRepo::discover(&hint)).await;
        match result {
            Ok(Ok(repo)) => {
                tracing::info!(root = ?repo.root, "LID repo discovered");
                let _ = self.repo_root.get_or_init(|| async { repo.root.clone() }).await;
                let arc = Arc::new(repo);
                *self.repo.write().await = Some(Arc::clone(&arc));
                Some(arc)
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "LID repo discovery failed");
                None
            }
            Err(e) => {
                tracing::warn!(error = %e, "LID repo discovery task panicked");
                None
            }
        }
    }

    /// Reload the repo from the cached root. On any error the existing
    /// snapshot is preserved and a warning is emitted — this keeps the
    /// server stable while a file is being written mid-edit.
    async fn reload_repo(&self) -> Option<Arc<LidRepo>> {
        let Some(root) = self.repo_root.get().cloned() else {
            tracing::debug!("reload skipped: repo root not yet known");
            return self.repo.read().await.clone();
        };

        let result = tokio::task::spawn_blocking(move || LidRepo::discover(&root)).await;
        match result {
            Ok(Ok(new_repo)) => {
                tracing::info!(root = ?new_repo.root, "LID repo reloaded");
                let arc = Arc::new(new_repo);
                *self.repo.write().await = Some(Arc::clone(&arc));
                Some(arc)
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "repo reload failed — keeping previous snapshot");
                self.repo.read().await.clone()
            }
            Err(e) => {
                tracing::warn!(error = %e, "repo reload task panicked — keeping previous snapshot");
                self.repo.read().await.clone()
            }
        }
    }

    // ── Diagnostics ────────────────────────────────────────────────────────────

    /// Compute and publish diagnostics for `uri` using `text` as the
    /// source of truth. No-ops when no LID repo is reachable.
    async fn publish_diagnostics_for(&self, uri: Url, version: Option<i32>, text: &str) {
        let Some(repo) = self.get_repo(&uri).await else {
            return;
        };
        let diagnostics = if uri.path().ends_with("-specs.md") {
            handlers::diagnostics::diagnostics_for_spec_buffer(&repo, text)
        } else {
            handlers::diagnostics::diagnostics_for_buffer(&repo, text)
        };
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    /// Re-publish diagnostics for every document currently open in the
    /// editor. Called after a repo reload so stale squiggles are
    /// refreshed without waiting for the user to re-type something.
    async fn republish_all_diagnostics(&self, repo: &LidRepo) {
        for doc in self.store.snapshot_all() {
            let Ok(uri) = Url::parse(&doc.uri) else {
                continue;
            };
            let diagnostics = if uri.path().ends_with("-specs.md") {
                handlers::diagnostics::diagnostics_for_spec_buffer(repo, &doc.text)
            } else {
                handlers::diagnostics::diagnostics_for_buffer(repo, &doc.text)
            };
            self.client
                .publish_diagnostics(uri, diagnostics, Some(doc.version))
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LidServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![" ".into(), ",".into()]),
                ..CompletionOptions::default()
            }),
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

        // Register file-system watchers so the server learns about
        // changes made outside the editor (AI agents, git, scripts).
        let watchers = [
            "**/*.md",
            "**/index.yaml",
            "**/*.rs",
            "**/*.ts",
            "**/*.tsx",
            "**/*.js",
            "**/*.jsx",
            "**/*.py",
            "**/*.go",
            "**/*.java",
            "**/*.scala",
        ]
        .iter()
        .map(|glob| FileSystemWatcher {
            glob_pattern: GlobPattern::String((*glob).to_owned()),
            kind: None, // defaults to Create | Change | Delete
        })
        .collect::<Vec<_>>();

        let opts = DidChangeWatchedFilesRegistrationOptions { watchers };
        if let Ok(opts_value) = serde_json::to_value(opts) {
            let reg = Registration {
                id: "lid-file-watcher".to_owned(),
                method: "workspace/didChangeWatchedFiles".to_owned(),
                register_options: Some(opts_value),
            };
            if let Err(e) = self.client.register_capability(vec![reg]).await {
                tracing::warn!(error = %e, "could not register file watchers");
            }
        }
    }

    // ── Text document lifecycle ─────────────────────────────────────────────

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
        // Publish diagnostics against the *current* repo without reloading —
        // the user may be mid-keystroke and the on-disk state is stale.
        self.publish_diagnostics_for(uri, Some(version), &text)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        tracing::debug!(uri = %uri, "did_close");
        let _ = self.store.remove(uri.as_str());
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    /// On save: reload the repo from disk so that any changes to spec
    /// files, the index, or source `@spec` annotations are picked up
    /// immediately. If the reload fails (e.g. the file is still being
    /// written by a tool), the previous snapshot is silently kept.
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        tracing::debug!(uri = %uri, "did_save");
        if let Some(repo) = self.reload_repo().await {
            self.republish_all_diagnostics(&repo).await;
        }
    }

    /// External file change (AI agent, git, script). Reload the repo
    /// and refresh diagnostics for every open buffer so squiggles stay
    /// accurate without the user having to touch anything.
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let paths: Vec<_> = params
            .changes
            .iter()
            .filter_map(|e| e.uri.to_file_path().ok())
            .collect();
        tracing::debug!(?paths, "did_change_watched_files");
        if let Some(repo) = self.reload_repo().await {
            self.republish_all_diagnostics(&repo).await;
        }
    }

    // ── Feature handlers ────────────────────────────────────────────────────

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(doc) = self.store.get(uri.as_str()) else {
            return Ok(None);
        };
        let Some(repo) = self.get_repo(&uri).await else {
            return Ok(None);
        };
        Ok(handlers::hover::hover_at_position(&repo, &doc.text, position))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let Some(doc) = self.store.get(uri.as_str()) else {
            return Ok(None);
        };
        let Some(repo) = self.get_repo(&uri).await else {
            return Ok(None);
        };
        Ok(handlers::completion::completions_at_position(
            &repo,
            uri.as_str(),
            &doc.text,
            position,
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
        let Some(repo) = self.get_repo(&uri).await else {
            return Ok(None);
        };
        Ok(handlers::definition::definition_at_position(
            &repo, &doc.text, position,
        ))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;
        let Some(doc) = self.store.get(uri.as_str()) else {
            return Ok(None);
        };
        let Some(repo) = self.get_repo(&uri).await else {
            return Ok(None);
        };
        Ok(handlers::references::references_at_position(
            &repo,
            &doc.text,
            position,
            include_declaration,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;
        let Some(doc) = self.store.get(uri.as_str()) else {
            return Ok(None);
        };
        Ok(handlers::rename::prepare_rename_at_position(
            &doc.text, position,
        ))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(doc) = self.store.get(uri.as_str()) else {
            return Ok(None);
        };
        let Some(repo) = self.get_repo(&uri).await else {
            return Ok(None);
        };
        match handlers::rename::rename_at_position(
            &repo,
            &self.store,
            &doc.text,
            position,
            &new_name,
        ) {
            Ok(edit) => Ok(Some(edit)),
            Err(e) => Err(tower_lsp::jsonrpc::Error::invalid_params(e.to_string())),
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let Some(repo) = self.repo.read().await.clone() else {
            return Ok(None);
        };
        Ok(Some(handlers::workspace_symbol::list_symbols(
            &repo,
            &params.query,
        )))
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("shutdown requested");
        Ok(())
    }
}
