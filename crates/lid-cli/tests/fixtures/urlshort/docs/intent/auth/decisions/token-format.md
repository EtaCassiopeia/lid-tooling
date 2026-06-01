# Auth Tokens: Opaque Random Strings, Not JWTs

Use cryptographically random opaque tokens stored in Redis rather than
self-contained JWTs for the URL-shortener's API key scheme.

## Context

The service issues API keys to clients that want to shorten URLs. We need tokens
that are revocable, cheap to validate, and simple to implement.

## Decision

Tokens are 32-byte random values (base64url-encoded, 43 chars) generated with a
CSPRNG at issuance and stored in Redis with a configurable TTL. Validation is a
single Redis `GET` — no signature verification, no clock-skew concern.

JWTs were considered and rejected: they are not revocable without a deny-list
(which re-introduces the Redis dependency anyway), and their flexibility is
unnecessary for this use-case.

## Consequences

- Revoking a token is a Redis `DEL` — immediate and reliable.
- Token validation latency is one network round-trip to Redis; acceptable given
  that the redirect endpoint (`GET /:alias`) does not require auth.
- If Redis is unavailable, token validation fails closed (requests are rejected).
  This is the desired behaviour for a rate-limited public API.
