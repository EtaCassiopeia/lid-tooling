---
prefix: STORAGE-REDIS-STORE
---

# EARS Specs: redis-store

**Implementing artifacts**: `src/redis_store.rs`, `tests/redis_store_test.rs`

## Behaviour

- [ ] **STORAGE-REDIS-STORE-001**: `RedisStore::new(config)` establishes a connection pool; `get` and `put` fail fast if the pool is exhausted.
- [ ] **STORAGE-REDIS-STORE-002**: After `put(alias, url)`, a subsequent `get(alias)` on any connection in the pool returns `Some(url)`.
- [ ] **STORAGE-REDIS-STORE-003**: `get` for an alias that does not exist in Redis returns `None` (not an error).
- [ ] **STORAGE-REDIS-STORE-004**: `RedisStore` satisfies the `Store` trait and can replace `InMemoryStore` at startup without any changes to `shortener-core` or `api`.
