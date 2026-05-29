# Arrow: shorten-endpoint

POST /shorten — accepts a JSON body `{ "url": "..." }`, delegates to `shortener-core`, returns 201 (new alias) or 200 (idempotent re-submission).

## Status

**MAPPED** — happy path and idempotency implemented; collision and error-body specs still open.

## References

### HLD
- `docs/high-level-design.md` § Key Design Decisions / Shorten response code

### LLD
- `docs/intent/api/shorten-endpoint-design.md`

### EARS
- `docs/intent/api/shorten-endpoint-specs.md` (5 specs, prefix `USH-POST-*`)

### Tests
- `tests/api_test.rs`

### Code
- `src/api.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-POST | USH-POST-001..005 | 3 | 2 | 0 |
