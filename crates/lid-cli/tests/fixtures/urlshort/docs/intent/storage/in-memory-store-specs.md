---
prefix: USH-IMEM
---

# EARS Specs: in-memory-store

**Implementing artifacts**: `src/storage.rs`, `tests/storage_test.rs`

## Core Behaviour

- [x] **USH-IMEM-001**: `InMemoryStore::new()` returns a store where `get` for any alias returns `None`.
- [x] **USH-IMEM-002**: After `put(alias, url)`, `get(alias)` returns `Some(url)`.
- [x] **USH-IMEM-003**: After `put(alias, url1)` followed by `put(alias, url2)`, `get(alias)` returns `Some(url2)`.
- [x] **USH-IMEM-004**: `InMemoryStore` can be cloned; both the original and the clone share the same underlying map.

## Concurrency

- [D] **USH-IMEM-005**: `InMemoryStore` does not degrade under high read concurrency when the map exceeds 100,000 entries (performance benchmark, not a correctness spec).
