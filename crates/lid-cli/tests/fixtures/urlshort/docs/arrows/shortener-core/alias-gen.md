# Arrow: alias-gen

Alias generation algorithm — SHA-256 of the input URL, first 7 base62 characters. Deterministic and collision-resistant at expected alias counts.

## Status

**OK** — all 6 specs implemented, reviewed, and audited. No open items.

## References

### HLD
- `docs/high-level-design.md` § Key Design Decisions / Alias length

### LLD
- `docs/intent/shortener-core/alias-gen-design.md`

### EARS
- `docs/intent/shortener-core/alias-gen-specs.md` (6 specs, prefix `USH-ALIAS-*`)

### Tests
- `tests/shortener_core_test.rs`

### Code
- `src/shortener_core.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-ALIAS | USH-ALIAS-001..006 | 6 | 0 | 0 |
