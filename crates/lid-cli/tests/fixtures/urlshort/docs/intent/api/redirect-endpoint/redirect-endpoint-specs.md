---
prefix: API-REDIRECT-ENDPOINT
---

# EARS Specs: redirect-endpoint

**Implementing artifacts**: `src/api.rs`, `tests/api_test.rs`

## Happy Path

- [x] **API-REDIRECT-ENDPOINT-001**: When GET /:alias is called with an existing alias, it returns 301 with a `Location` header set to the original URL.
- [x] **API-REDIRECT-ENDPOINT-002**: When GET /:alias is called with an alias that does not exist, it returns 404.

## Edge Cases

- [ ] **API-REDIRECT-ENDPOINT-003**: When GET /:alias is called with an alias containing characters outside `[0-9A-Za-z_]`, it returns 404.
- [ ] **API-REDIRECT-ENDPOINT-004**: When GET /:alias is called with an empty alias segment (`GET /`), the request is not routed to this handler (router-level concern).
