# Arrow: store-interface

The `Store` trait — `get(alias) → Option<Url>` and `put(alias, url) → ()`. All storage implementations must satisfy this contract.

## Status

**AUDITED** — trait design reviewed and sampled; all 5 specs implemented and verified.

## References

### HLD
- `docs/high-level-design.md` § Key Design Decisions / Storage interface ownership

### LLD
- `docs/intent/storage/store-interface/store-interface-design.md`

### EARS
- `docs/intent/storage/store-interface/store-interface-specs.md` (5 specs, prefix `STORAGE-STORE-INTERFACE-*`)

### Tests
- `tests/storage_test.rs`

### Code
- `src/storage.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All STORAGE-STORE-INTERFACE | STORAGE-STORE-INTERFACE-001..005 | 5 | 0 | 0 |
