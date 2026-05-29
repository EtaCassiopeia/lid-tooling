# Arrow: storage

Persistence abstraction for alias→URL pairs. Defines the `Store` trait that `shortener-core` depends on; provides an `InMemoryStore` implementation for v1.

## Status

**UNMAPPED** — LLD and EARS authored; implementation not started.

## References

### HLD
- `docs/high-level-design.md` § Approach / storage; § Key Design Decisions / Storage interface ownership; § Goal 2

### LLD
- `docs/intent/storage/storage-design.md`

### EARS
- `docs/intent/storage/storage-specs.md` (10 specs, prefix `USH-STORE-*`)

### Tests
- (none yet)

### Code
- (none yet)

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-STORE | USH-STORE-001..010 | 0 | 10 | 0 |
