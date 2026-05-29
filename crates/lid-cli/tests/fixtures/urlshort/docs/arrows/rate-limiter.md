# Arrow: rate-limiter

Sliding-window per-API-key throttle protecting POST /shorten. Trusted internal callers may bypass via a signed header.

## Status

**MAPPED** — sliding-window implementation shipped in PR #42; three specs implemented. USH-RATE-003 is out of sync with implementation (see Drift).

## References

### HLD
- `docs/high-level-design.md` § Approach / rate-limiter; § Key Design Decisions / Rate-limit algorithm

### LLD
- `docs/intent/rate-limiter/rate-limiter-design.md`

### EARS
- `docs/intent/rate-limiter/rate-limiter-specs.md` (5 specs, prefix `USH-RATE-*`)

### Tests
- `tests/rate_limiter_test.rs`

### Code
- `src/rate_limiter.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-RATE | USH-RATE-001..005 | 3 | 2 | 0 |
