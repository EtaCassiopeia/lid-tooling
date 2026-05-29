# Arrow: shortener-core

Domain logic for URL shortening — alias generation, collision handling, and the idempotency contract. Stateless; depends on the `storage` interface for persistence.

## Status

**MAPPED** — alias generation and basic collision done; validation and storage-interaction specs still open.

## References

### HLD
- `docs/high-level-design.md` § Approach / shortener-core; § Key Design Decisions / Alias length; § Key Design Decisions / Collision strategy

### LLD
- `docs/intent/shortener-core/shortener-core-design.md`

### EARS
- `docs/intent/shortener-core/shortener-core-specs.md` (13 specs, prefix `USH-CORE-*`)

### Tests
- `tests/shortener_core_test.rs`

### Code
- `src/shortener_core.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| Alias generation | USH-CORE-001..002 | 2 | 0 | 0 |
| Collision handling | USH-CORE-003..005 | 2 | 1 | 0 |
| Validation | USH-CORE-006..008 | 0 | 3 | 0 |
| Resolution | USH-CORE-009..010 | 1 | 1 | 0 |
| Storage interaction | USH-CORE-011..013 | 0 | 3 | 0 |
