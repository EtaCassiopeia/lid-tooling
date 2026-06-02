# Arrow: redirect-endpoint

GET /:alias — resolves the alias via `shortener-core` and returns 301 with a `Location` header. Returns 404 for unknown or malformed aliases.

## Status

**MAPPED** — redirect and 404 implemented; invalid-character handling still open.

## References

### HLD
- `docs/high-level-design.md` § Key Design Decisions / Redirect status

### LLD
- `docs/intent/api/redirect-endpoint/redirect-endpoint-design.md`

### EARS
- `docs/intent/api/redirect-endpoint/redirect-endpoint-specs.md` (4 specs, prefix `API-REDIRECT-ENDPOINT-*`)

### Tests
- `tests/api_test.rs`

### Code
- `src/api.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All API-REDIRECT-ENDPOINT | API-REDIRECT-ENDPOINT-001..004 | 2 | 2 | 0 |
