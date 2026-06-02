---
prefix: STORAGE
---

# EARS Specs: storage

**Implementing artifacts**: `src/storage.rs`, `tests/storage_test.rs`

## Interface Contract

- [x] **STORAGE-001**: When `get` is called with an alias that exists, it returns `Some(url)`.
- [x] **STORAGE-002**: When `get` is called with an alias that does not exist, it returns `None`.
- [x] **STORAGE-003**: When `put` is called with a new alias→url pair, subsequent `get` for that alias returns the stored URL.
- [x] **STORAGE-004**: When `put` is called with an alias that already exists, it overwrites the existing URL.

## InMemoryStore

- [x] **STORAGE-005**: `InMemoryStore::new()` returns an empty store where `get` always returns `None`.
- [x] **STORAGE-006**: `InMemoryStore` is safe to use from multiple threads concurrently (reads do not block other reads).
- [ ] **STORAGE-007**: Concurrent `put` operations on different aliases do not interfere with each other.
- [ ] **STORAGE-008**: Concurrent `put` and `get` for the same alias are serialized — `get` returns either the old or the new value, never a partial write.

## Interface Isolation

- [ ] **STORAGE-009**: `shortener-core` depends only on the `Store` trait, not on any concrete implementation type.
- [ ] **STORAGE-010**: A test-double `Store` implementation can be injected into `shortener-core` without modifying any production code.
