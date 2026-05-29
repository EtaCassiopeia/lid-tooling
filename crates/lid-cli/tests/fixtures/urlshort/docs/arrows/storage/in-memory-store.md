# Arrow: in-memory-store

`Arc<RwLock<HashMap>>` implementation of `Store`. Used in development and tests; the default store for v1 until Redis is provisioned.

## Status

**OK** — all specs implemented, audited, and verified. One spec deferred (resize-under-load behaviour).

## References

### HLD
- `docs/high-level-design.md` § Approach / storage

### LLD
- `docs/intent/storage/in-memory-store-design.md`

### EARS
- `docs/intent/storage/in-memory-store-specs.md` (5 specs, prefix `USH-IMEM-*`)

### Tests
- `tests/storage_test.rs`

### Code
- `src/storage.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-IMEM | USH-IMEM-001..005 | 4 | 0 | 1 |
