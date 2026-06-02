# Arrow: collision

Collision detection and retry. When the generated alias is already taken by a different URL, appends `_N` (N=1..5) and retries. Returns `ShortenError::CollisionLimit` if all candidates are exhausted.

## Status

**MAPPED** — basic retry logic implemented; exponential back-off and limit-exceeded behaviour still open.

## References

### HLD
- `docs/high-level-design.md` § Key Design Decisions / Collision strategy

### LLD
- `docs/intent/shortener-core/collision/collision-design.md`

### EARS
- `docs/intent/shortener-core/collision/collision-specs.md` (5 specs, prefix `SHORTENER-CORE-COLLISION-*`)

### Tests
- `tests/shortener_core_test.rs`

### Code
- `src/shortener_core.rs`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All SHORTENER-CORE-COLLISION | SHORTENER-CORE-COLLISION-001..005 | 1 | 4 | 0 |
