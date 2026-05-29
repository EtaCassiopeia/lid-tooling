# Arrow: api

HTTP surface for the URL shortener — POST /shorten, GET /:alias, and GET /health. Input validation, error mapping, and middleware integration. No domain logic; delegates all decisions to `shortener-core`.

## Status

**MAPPED** — all three endpoints specified; shorten and redirect partially implemented.

## References

### HLD
- `docs/high-level-design.md` § Approach / api; § Key Design Decisions / Redirect status; § Key Design Decisions / Shorten response code

### LLD
- `docs/intent/api/api-design.md`

### EARS
- `docs/intent/api/api-specs.md` (14 specs, prefix `USH-API-*`)

### Tests
- `tests/api_test.rs`

### Code
- `src/api.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| POST /shorten | USH-API-001..008 | 6 | 2 | 0 |
| GET /:alias | USH-API-009..011 | 2 | 1 | 0 |
| Error responses | USH-API-012..014 | 0 | 3 | 0 |
