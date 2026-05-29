# High-Level Design: URL Shortener

A URL shortener service. Given a long URL it returns a short alias; given that alias it redirects to the original URL. The design separates infrastructure, domain logic, HTTP surface, and cross-cutting concerns into distinct components with explicit interface boundaries.

## Goals

1. **Shorten URLs reliably.** Alias generation is deterministic for the same long URL and idempotent on re-submission.
2. **Redirect fast.** Lookup latency is the user-facing metric; alias storage must support O(1) reads.
3. **Stay a minimum system.** No analytics, no custom aliases, no expiry in v1. Each addition must justify itself against this goal.
4. **Protect the service.** All write paths are authenticated and rate-limited.

## Approach

Five logical components, separated by clear interface boundaries:

1. **storage** — persistence abstraction: CRUD over alias→URL pairs. The `Store` trait is owned here; implementations (in-memory, Redis) are injected at startup. Two sub-components: `store-interface` (the trait) and `in-memory-store` (v1 implementation). `redis-store` is planned for Q3 2026.
2. **shortener-core** — domain logic: alias generation, collision handling, idempotency contract. Stateless; depends on `storage`. Split into `alias-gen` (the generation algorithm) and `collision` (retry and back-off).
3. **api** — HTTP surface: POST /shorten, GET /:alias, GET /health. Input validation, error mapping. Depends on `shortener-core`, `auth`, and `rate-limiter`. Split by endpoint: `shorten-endpoint`, `redirect-endpoint`, `health-endpoint`.
4. **auth** — API-key middleware. Validates HMAC-SHA256 bearer tokens. All mutating endpoints require authentication; GET /:alias is public.
5. **rate-limiter** — sliding-window per-API-key throttle. Protects POST /shorten; bypass allowed for trusted internal callers.

Future: **analytics** — click-count aggregation and per-alias redirect stats.

## Tenets

- **Interfaces at component boundaries.** `api` never calls `storage` directly; `shortener-core` is the only domain mediator.
- **Tests before code.** EARS specs are the contract; code that passes the specs is done.
- **Simplicity over flexibility.** Absorb into existing components before adding new ones.

## Key Design Decisions

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Alias length | 7 chars (base62) | shorter/longer | ~3.5T aliases before collision pressure; readable without encoding |
| Collision strategy | Append `_N`, retry up to 5× | random regeneration | Deterministic, bounded, testable |
| Storage interface ownership | `storage` component | `shortener-core` owns it | Keeps storage swappable without touching domain logic |
| Redirect status | 301 Permanent | 302 Temporary | Aliases are immutable; permanent redirect enables browser/CDN caching |
| Shorten response code | 201 Created / 200 OK | always 200 | 201 signals new alias; 200 signals idempotent re-submission — distinguishable by client |
| Auth scheme | HMAC-SHA256 API keys | OAuth2, mTLS | Stateless verification; no token-store dependency |
| Rate-limit algorithm | Sliding window | Token bucket, fixed window | More accurate than fixed window; avoids token-bucket spec/implementation divergence |

## Analytics

Click-count aggregation is explicitly out of scope for v1. A future `analytics` component will capture per-alias redirect events. Storage backend (ClickHouse vs TimescaleDB) to be decided in a separate HLD update before EARS are authored.
