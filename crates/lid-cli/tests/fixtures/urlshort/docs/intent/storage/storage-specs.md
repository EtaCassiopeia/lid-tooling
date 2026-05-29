# EARS Specs: storage

**Implementing artifacts**: `src/storage.rs`, `tests/storage_test.rs`

## Interface Contract

- [x] **USH-STORE-001**: When `get` is called with an alias that exists, it returns `Some(url)`.
- [x] **USH-STORE-002**: When `get` is called with an alias that does not exist, it returns `None`.
- [x] **USH-STORE-003**: When `put` is called with a new alias→url pair, subsequent `get` for that alias returns the stored URL.
- [x] **USH-STORE-004**: When `put` is called with an alias that already exists, it overwrites the existing URL.

## InMemoryStore

- [x] **USH-STORE-005**: `InMemoryStore::new()` returns an empty store where `get` always returns `None`.
- [x] **USH-STORE-006**: `InMemoryStore` is safe to use from multiple threads concurrently (reads do not block other reads).
- [ ] **USH-STORE-007**: Concurrent `put` operations on different aliases do not interfere with each other.
- [ ] **USH-STORE-008**: Concurrent `put` and `get` for the same alias are serialized — `get` returns either the old or the new value, never a partial write.

## Interface Isolation

- [ ] **USH-STORE-009**: `shortener-core` depends only on the `Store` trait, not on any concrete implementation type.
- [ ] **USH-STORE-010**: A test-double `Store` implementation can be injected into `shortener-core` without modifying any production code.
