---
prefix: USH-ANLY
---

# EARS Specs: analytics

**Implementing artifacts**: `src/analytics.rs`, `tests/analytics_test.rs`

## Event Capture

- [ ] **USH-ANLY-001**: When GET /:alias returns 301, an analytics event `{ alias, timestamp, user_agent }` is emitted to the analytics channel without blocking the redirect response.
- [ ] **USH-ANLY-002**: When the analytics channel is full, the event is dropped and a counter metric is incremented — the redirect response is never delayed.

## Aggregation

- [ ] **USH-ANLY-003**: The analytics worker exposes a query `click_count(alias) → u64` that returns the total number of redirects for the given alias.
- [ ] **USH-ANLY-004**: `click_count` returns 0 for aliases that have never been redirected (not an error).
