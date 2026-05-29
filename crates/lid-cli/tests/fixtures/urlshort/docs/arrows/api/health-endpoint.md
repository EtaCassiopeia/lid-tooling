# Arrow: health-endpoint

GET /health — returns 200 `{ "status": "ok" }`. Used by load-balancer health checks and readiness probes.

## Status

**OK** — all 3 specs implemented and audited.

## References

### HLD
- `docs/high-level-design.md` § Approach / api

### LLD
- `docs/intent/api/health-endpoint-design.md`

### EARS
- `docs/intent/api/health-endpoint-specs.md` (3 specs, prefix `USH-HLTH-*`)

### Tests
- `tests/api_test.rs`

### Code
- `src/api.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All USH-HLTH | USH-HLTH-001..003 | 3 | 0 | 0 |
