---
prefix: ANALYTICS
---

# EARS Specs: analytics

**Implementing artifacts**: `src/analytics.rs`, `tests/analytics_test.rs`

## Event Capture

- [ ] **ANALYTICS-001**: When GET /:alias returns 301, an analytics event `{ alias, timestamp, user_agent }` is emitted to the analytics channel without blocking the redirect response.
- [ ] **ANALYTICS-002**: When the analytics channel is full, the event is dropped and a counter metric is incremented — the redirect response is never delayed.

## Aggregation

- [ ] **ANALYTICS-003**: The analytics worker exposes a query `click_count(alias) → u64` that returns the total number of redirects for the given alias.
- [ ] **ANALYTICS-004**: `click_count` returns 0 for aliases that have never been redirected (not an error).
