# LLD: rate-limiter

**Created**: 2026-05-01  
**Status**: Partial — sliding-window implementation shipped in PR #42

## Context

`rate-limiter` throttles POST /shorten per API key. The original spec described a token-bucket model; the implementation uses a sliding window. The two are behaviourally different under burst traffic. RATE-LIMITER-003 must be updated to reflect the actual implementation.

## Algorithm (Sliding Window)

For each `key_id`, maintain a sorted list of request timestamps in Redis. On each request:
1. Remove timestamps older than the window (1 minute).
2. Count remaining timestamps.
3. If count ≥ limit (100 req/min), reject with 429.
4. Otherwise append the current timestamp and allow.

## Trusted Bypass

Internal callers may include `X-Rate-Limit-Bypass: {bypass_token}` to skip throttling. The bypass token is validated the same way as API keys.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Algorithm | Sliding window | Token bucket (original spec) | Sliding window is more accurate near window edges; adopted in PR #42 before spec was updated |
| Window duration | 60 seconds | 30s, 5 minutes | Balance between burst tolerance and abuse prevention |
| Limit | 100 req/min per key | 50, 1000 | Conservative default; operators can override via config |
