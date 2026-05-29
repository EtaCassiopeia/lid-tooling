# LLD: auth

**Created**: 2026-05-29  
**Status**: Draft — no implementation started

## Context

`auth` provides an axum middleware layer that validates API keys on mutating endpoints. GET /:alias is public; POST /shorten requires authentication. Keys are issued out-of-band and stored in the service configuration.

## Scheme

API keys are HMAC-SHA256 tokens: `HMAC-SHA256(secret, key_id + ":" + timestamp_day)`. The timestamp is truncated to day granularity so keys rotate daily without client coordination.

```
Authorization: Bearer {key_id}:{hmac_hex}
```

The middleware:
1. Parses the `Authorization: Bearer` header.
2. Extracts `key_id` and `hmac_hex`.
3. Looks up the shared secret for `key_id` from config.
4. Recomputes `HMAC-SHA256(secret, key_id + ":" + today)`.
5. Returns 401 if the header is absent, malformed, unknown key_id, or HMAC mismatch.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Auth scheme | HMAC-SHA256 daily tokens | OAuth2, mTLS, static keys | Stateless verification; no token-store needed; daily rotation limits blast radius |
| Header format | `Authorization: Bearer` | Custom `X-Api-Key` | Standard HTTP; works with existing tooling |
| GET /alias auth | None (public) | Required | Redirect must be cacheable by CDN; adding auth breaks caching |
