# LLD: shortener-core

**Created**: 2026-05-29
**Status**: Pending implementation

## Context

`shortener-core` owns the domain logic for URL shortening: alias generation, collision handling, and the idempotency contract. It is the only component that knows about alias structure and business rules; `api` and `storage` are downstream consumers.

## HLD Trace

- **Goal 1** — alias generation is deterministic for the same input URL and idempotent on re-submission.
- **Goal 2** — collision handling keeps the alias→URL lookup single-step (no lookup table indirection).
- **Approach § shortener-core** — stateless domain logic, depends on `storage` interface for persistence.
- **Key Design Decisions / Alias length** — 7-char base62.
- **Key Design Decisions / Collision strategy** — append counter, up to 5 retries.

## Component Responsibilities

1. **Alias generation.** Hash the input URL (SHA-256, first 7 base62 chars). If the candidate is taken and maps to a different URL, append `_N` for N=1..5. If all candidates are taken, return an error.
2. **Idempotency.** If the input URL already has an alias, return the existing alias without creating a new record.
3. **Validation.** Reject input URLs that are not HTTP/HTTPS, exceed 2048 chars, or contain control characters.

## Interface Contract

```
shorten(url: Url) -> Result<Alias, ShortenError>
resolve(alias: Alias) -> Result<Url, ResolveError>
```

`shorten` is the only write path. `resolve` is read-only and calls `storage.get(alias)` directly.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Hash function | SHA-256, first 7 base62 chars | UUID, random bytes | Deterministic → idempotent; base62 → URL-safe without encoding |
| Max retries | 5 | unbounded / 3 / 10 | Bounds worst-case latency; collision probability at 3.5T aliases makes 5 retries astronomically safe |
| Validation location | shortener-core | api layer only | Domain invariant, not HTTP concern — belongs here regardless of transport |
