---
prefix: USH-SITF
---

# EARS Specs: store-interface

**Implementing artifacts**: `src/storage.rs`, `tests/storage_test.rs`

## Trait Contract

- [x] **USH-SITF-001**: Any type implementing `Store` must provide `get(alias: &str) -> Option<String>`.
- [x] **USH-SITF-002**: Any type implementing `Store` must provide `put(alias: &str, url: &str)`.
- [x] **USH-SITF-003**: After `put(alias, url)`, a subsequent `get(alias)` returns `Some(url)`.
- [x] **USH-SITF-004**: `Store` implementations must be `Send + Sync` to support concurrent access from async tasks.
- [x] **USH-SITF-005**: A `Box<dyn Store>` or `Arc<dyn Store>` can be passed to `shortener-core` without the core depending on any concrete storage type.
