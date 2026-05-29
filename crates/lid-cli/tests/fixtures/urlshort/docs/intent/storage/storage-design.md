# LLD: storage

**Created**: 2026-05-29
**Status**: Pending implementation

## Context

`storage` owns the persistence abstraction for alias→URL pairs. It defines the interface that `shortener-core` depends on, and provides at least one concrete implementation. The implementation is injected at startup; `shortener-core` never imports a concrete implementation directly.

## HLD Trace

- **Approach § storage** — persistence abstraction; CRUD over alias→URL pairs; interface owned here.
- **Key Design Decisions / Storage interface ownership** — `storage` owns the interface, not `shortener-core`.
- **Goal 2** — O(1) reads required; in-memory HashMap and Redis both satisfy this.

## Interface

```
trait Store {
    fn get(alias: &Alias) -> Option<Url>;
    fn put(alias: Alias, url: Url) -> Result<(), StoreError>;
}
```

`get` is infallible by design — a missing key returns `None`, not an error. Only infrastructure failures (connection loss, OOM) are errors.

## Implementations (v1)

- **InMemoryStore** — `HashMap<Alias, Url>` behind a `RwLock`. Sufficient for testing and single-process deployments.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| `get` returns `Option` not `Result` | `Option<Url>` | `Result<Option<Url>, StoreError>` | Missing key is not an error; infrastructure errors are separate |
| v1 implementation | In-memory only | Redis, Postgres | YAGNI; swap in by injecting a different `Store` impl |
| Interface location | `storage` crate | `shortener-core` crate | Keeps `shortener-core` free of storage-specific types |
