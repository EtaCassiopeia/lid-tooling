# LLD: store-interface

**Created**: 2026-04-10  
**Status**: Implemented and audited

## Context

`store-interface` defines the `Store` trait that all persistence implementations must satisfy. Owning the trait here (not in `shortener-core`) keeps storage swappable without touching domain logic.

## Trait Definition

```rust
pub trait Store: Send + Sync {
    fn get(&self, alias: &str) -> Option<String>;
    fn put(&self, alias: &str, url: &str);
}
```

`Send + Sync` are required because the store is shared across async tasks.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Trait ownership | `storage` crate | `shortener-core` crate | Avoids circular dependency; storage layer is infrastructure, not domain |
| `put` return type | `()` (infallible) | `Result<(), StoreError>` | In-memory store never fails; error handling deferred to RedisStore design |
| Async vs sync | Sync | Async (`async fn`) | Sync is simpler; `spawn_blocking` wraps sync calls in async contexts without trait-object complexity |
