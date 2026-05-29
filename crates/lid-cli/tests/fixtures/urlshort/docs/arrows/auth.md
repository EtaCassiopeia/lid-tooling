# Arrow: auth

API-key authentication middleware. Validates HMAC-SHA256 bearer tokens on all mutating endpoints (POST /shorten). GET /:alias is intentionally public.

## Status

**UNMAPPED** — LLD and EARS authored; no implementation started.

## References

### HLD
- `docs/high-level-design.md` § Approach / auth; § Key Design Decisions / Auth scheme

### LLD
- `docs/intent/auth/auth-design.md`

### EARS
- `docs/intent/auth/auth-specs.md` (6 specs, prefix `USH-AUTH-*`)

### Tests
- (none yet)

### Code
- (none yet)

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-AUTH | USH-AUTH-001..006 | 0 | 6 | 0 |
