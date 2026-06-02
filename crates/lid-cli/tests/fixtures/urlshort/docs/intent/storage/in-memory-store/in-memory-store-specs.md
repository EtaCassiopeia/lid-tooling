---
prefix: STORAGE-IN-MEMORY-STORE
---

# EARS Specs: in-memory-store

**Implementing artifacts**: `src/storage.rs`, `tests/storage_test.rs`

## Core Behaviour

- [x] **STORAGE-IN-MEMORY-STORE-001**: `InMemoryStore::new()` returns a store where `get` for any alias returns `None`.
- [x] **STORAGE-IN-MEMORY-STORE-002**: After `put(alias, url)`, `get(alias)` returns `Some(url)`.
- [x] **STORAGE-IN-MEMORY-STORE-003**: After `put(alias, url1)` followed by `put(alias, url2)`, `get(alias)` returns `Some(url2)`.
- [x] **STORAGE-IN-MEMORY-STORE-004**: `InMemoryStore` can be cloned; both the original and the clone share the same underlying map.

## Concurrency

- [D] **STORAGE-IN-MEMORY-STORE-005**: `InMemoryStore` does not degrade under high read concurrency when the map exceeds 100,000 entries (performance benchmark, not a correctness spec).
