# Arrow: shortener-core

Domain logic for URL shortening — alias generation, collision handling, and idempotency contract. Stateless; depends on `storage` interface for persistence.

## Status

**UNMAPPED** — LLD and EARS authored; implementation not started.

## References

### HLD
- `docs/high-level-design.md` § Approach / shortener-core; § Key Design Decisions / Alias length; § Key Design Decisions / Collision strategy

### LLD
- `docs/intent/shortener-core/shortener-core-design.md`

### EARS
- `docs/intent/shortener-core/shortener-core-specs.md` (13 specs, prefix `USH-CORE-*`)

### Tests
- (none yet)

### Code
- (none yet)

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-CORE | USH-CORE-001..013 | 0 | 13 | 0 |
