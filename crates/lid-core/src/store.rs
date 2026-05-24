//! Concurrent in-memory document store keyed by LSP URI.
//!
//! The LSP server holds a `DocStore` for the lifetime of the session.
//! Each open editor buffer becomes one entry; `didChange` notifications
//! replace the entry in place and `didClose` removes it. Readers
//! (hover, diagnostics, definition) `get` an `Arc<Document>` snapshot
//! that's safe to hold across `.await` points without blocking
//! concurrent writers.
//!
//! The store doesn't parse — it just stores text. Parsing happens in
//! handlers on demand, which keeps the hot path during `didChange`
//! constant-time regardless of document size.

use std::sync::Arc;

use dashmap::DashMap;

/// One open editor buffer.
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Arc<str>,
    /// `Arc<String>` rather than `String` so a reader can clone the
    /// snapshot cheaply and the writer can replace the entry without
    /// invalidating the reader's view.
    pub text: Arc<String>,
    /// LSP sync version number; useful for filtering stale responses.
    pub version: i32,
}

/// Concurrent URI → [`Document`] map.
#[derive(Debug, Default)]
pub struct DocStore {
    docs: DashMap<Arc<str>, Arc<Document>>,
}

impl DocStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace the document at `uri`. Returns the resulting
    /// `Arc<Document>` so callers can hand it straight to a handler
    /// without a second `get` lookup. The return is intentionally not
    /// `#[must_use]` — the side effect (storing) is the point, and
    /// most `didChange` handlers discard the value.
    pub fn upsert(&self, uri: impl Into<Arc<str>>, text: String, version: i32) -> Arc<Document> {
        let uri: Arc<str> = uri.into();
        let doc = Arc::new(Document {
            uri: Arc::clone(&uri),
            text: Arc::new(text),
            version,
        });
        self.docs.insert(uri, Arc::clone(&doc));
        doc
    }

    /// Snapshot the current document for `uri`. Returns `None` when
    /// the URI is unknown — typically because the editor never sent
    /// `didOpen` or already sent `didClose`.
    #[must_use]
    pub fn get(&self, uri: &str) -> Option<Arc<Document>> {
        self.docs.get(uri).map(|entry| Arc::clone(entry.value()))
    }

    /// Remove the document at `uri`, returning the dropped value if
    /// present. Called by `didClose`.
    #[must_use]
    pub fn remove(&self, uri: &str) -> Option<Arc<Document>> {
        self.docs.remove(uri).map(|(_, v)| v)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::*;

    #[test]
    fn empty_store_returns_none() {
        let store = DocStore::new();
        assert!(store.get("file:///missing").is_none());
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn upsert_then_get_round_trips() {
        let store = DocStore::new();
        store.upsert("file:///a", "hello".to_owned(), 1);
        let doc = store.get("file:///a").unwrap();
        assert_eq!(&*doc.text, "hello");
        assert_eq!(doc.version, 1);
        assert_eq!(&*doc.uri, "file:///a");
    }

    #[test]
    fn upsert_replaces_existing_entry() {
        let store = DocStore::new();
        store.upsert("file:///a", "v1".to_owned(), 1);
        store.upsert("file:///a", "v2".to_owned(), 2);
        let doc = store.get("file:///a").unwrap();
        assert_eq!(&*doc.text, "v2");
        assert_eq!(doc.version, 2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn remove_drops_entry() {
        let store = DocStore::new();
        store.upsert("file:///a", "hello".to_owned(), 1);
        let removed = store.remove("file:///a").unwrap();
        assert_eq!(&*removed.text, "hello");
        assert!(store.get("file:///a").is_none());
    }

    #[test]
    fn remove_returns_none_for_unknown_uri() {
        let store = DocStore::new();
        assert!(store.remove("file:///missing").is_none());
    }

    #[test]
    fn reader_snapshot_outlives_concurrent_writer() {
        // A reader holding an Arc<Document> sees the old text even
        // after a writer replaces the entry — this is the property
        // that makes the store safe for use across `.await` points.
        let store = DocStore::new();
        store.upsert("file:///a", "old".to_owned(), 1);
        let snapshot = store.get("file:///a").unwrap();
        store.upsert("file:///a", "new".to_owned(), 2);
        assert_eq!(&*snapshot.text, "old");
        assert_eq!(&*store.get("file:///a").unwrap().text, "new");
    }

    #[test]
    fn concurrent_upserts_do_not_lose_entries() {
        let store = Arc::new(DocStore::new());
        let handles: Vec<_> = (0..16)
            .map(|i| {
                let store = Arc::clone(&store);
                thread::spawn(move || {
                    store.upsert(format!("file:///doc-{i}"), format!("text-{i}"), 1);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(store.len(), 16);
        for i in 0..16 {
            let doc = store.get(&format!("file:///doc-{i}")).unwrap();
            assert_eq!(&*doc.text, &format!("text-{i}"));
        }
    }
}
