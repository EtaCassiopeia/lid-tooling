# LLD: in-memory-store

**Created**: 2026-04-10  
**Status**: Implemented and audited

## Context

`InMemoryStore` is the v1 production store and the reference implementation of `Store`. It uses an `Arc<RwLock<HashMap<String, String>>>` so multiple readers can proceed concurrently while writes are serialized.

## Implementation

```rust
pub struct InMemoryStore {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl Store for InMemoryStore {
    fn get(&self, alias: &str) -> Option<String> {
        self.data.read().unwrap().get(alias).cloned()
    }
    fn put(&self, alias: &str, url: &str) {
        self.data.write().unwrap().insert(alias.to_owned(), url.to_owned());
    }
}
```

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Synchronization | `RwLock` | `Mutex`, `DashMap` | `RwLock` allows concurrent reads; alias reads dominate writes |
| Wrap in `Arc` | Yes | No (caller manages Arc) | Simplifies cloning the store into multiple tasks |
| Resize-under-load | Not addressed | Pre-size HashMap | Low priority for v1 volumes; deferred to redis-store migration |
