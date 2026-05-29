# LLD: collision

**Created**: 2026-04-20  
**Status**: Partial — basic retry shipped; back-off and limit handling pending

## Context

When `alias-gen` produces an alias that is already stored for a *different* URL, `shortener-core` must find an alternative. This component owns that retry loop.

## Strategy

1. Attempt with the base alias (no suffix).
2. If taken by a different URL, try `alias_1`, `alias_2`, … up to `alias_5`.
3. If all five are taken, return `ShortenError::CollisionLimit`.
4. If any candidate is taken by the *same* URL (idempotent case), return that alias immediately.

Each retry calls `storage.get` once, then `storage.put` once on the first free slot. At most 6 `storage.get` calls and 1 `storage.put` per `shorten` invocation.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Retry suffix scheme | `_{N}` append | random regeneration | Deterministic; caller can reproduce the same result without state |
| Max retries | 5 | 3, 10, unbounded | Bounds worst-case latency; at 3.5T total aliases collision probability makes 5 retries astronomically safe |
| Back-off | None yet (planned) | Exponential back-off | High-concurrency write spikes may cause spurious CollisionLimit; back-off planned in next sprint |
