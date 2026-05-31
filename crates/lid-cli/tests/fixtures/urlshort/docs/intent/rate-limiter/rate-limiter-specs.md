---
prefix: USH-RATE
---

# EARS Specs: rate-limiter

**Implementing artifacts**: `src/rate_limiter.rs`, `tests/rate_limiter_test.rs`

## Throttling

- [x] **USH-RATE-001**: When a key has made fewer than 100 requests in the last 60 seconds, the next request is allowed through.
- [x] **USH-RATE-002**: When a key has made 100 or more requests in the last 60 seconds, the middleware returns 429 with `{ "error": "rate limit exceeded" }`.
- [ ] **USH-RATE-003**: The rate limit uses a token-bucket algorithm with a refill rate of 100 tokens per minute. ← DRIFT: implementation uses sliding window, not token bucket.
- [x] **USH-RATE-004**: The rate limit is tracked independently per API key — one key hitting the limit does not affect other keys.

## Bypass

- [ ] **USH-RATE-005**: When a request includes a valid `X-Rate-Limit-Bypass` header, the middleware allows it through regardless of the current count for that key.
