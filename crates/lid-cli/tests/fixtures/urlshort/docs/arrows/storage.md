# Arrow: storage

Persistence abstraction for alias→URL pairs. Owns the `Store` trait and its implementations. `shortener-core` depends on this interface; concrete implementations are injected at startup.

## Status

**MAPPED** — interface designed and audited; `InMemoryStore` shipped. `RedisStore` planned for Q3 2026.

## References

### HLD
- `docs/high-level-design.md` § Approach / storage; § Key Design Decisions / Storage interface ownership

### LLD
- `docs/intent/storage/storage-design.md`

### EARS
- `docs/intent/storage/storage-specs.md` (10 specs, prefix `USH-STORE-*`)

### Tests
- `tests/storage_test.rs`

### Code
- `src/storage.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| Interface contract | USH-STORE-001..004 | 4 | 0 | 0 |
| InMemoryStore | USH-STORE-005..008 | 2 | 2 | 0 |
| Interface isolation | USH-STORE-009..010 | 0 | 2 | 0 |
