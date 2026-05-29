# Arrow: api

HTTP surface for the URL shortener — POST /shorten and GET /:alias endpoints, input validation, and error mapping. No business logic; delegates all domain decisions to `shortener-core`.

## Status

**UNMAPPED** — LLD and EARS authored; implementation not started.

## References

### HLD
- `docs/high-level-design.md` § Approach / api; § Key Design Decisions / Redirect status; § Key Design Decisions / Shorten response code

### LLD
- `docs/intent/api/api-design.md`

### EARS
- `docs/intent/api/api-specs.md` (14 specs, prefix `USH-API-*`)

### Tests
- (none yet)

### Code
- (none yet)

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-API | USH-API-001..014 | 0 | 14 | 0 |
