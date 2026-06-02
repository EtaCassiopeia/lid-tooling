---
prefix: API
---

# EARS Specs: api

**Implementing artifacts**: `src/api.rs`, `tests/api_test.rs`

## POST /shorten

- [x] **API-001**: When POST /shorten is called with a valid URL, it returns 201 with `{ "alias": "...", "short_url": "..." }`.
- [x] **API-002**: When POST /shorten is called with a URL that already has an alias, it returns 200 with the existing alias (idempotent re-submission).
- [x] **API-003**: When POST /shorten is called with a non-HTTP/HTTPS URL, it returns 400.
- [x] **API-004**: When POST /shorten is called with a URL exceeding 2048 characters, it returns 400.
- [x] **API-005**: When POST /shorten is called with a malformed JSON body, it returns 400.
- [x] **API-006**: When POST /shorten is called with a missing `url` field, it returns 400.
- [ ] **API-007**: When POST /shorten results in a collision limit error, it returns 409.
- [ ] **API-008**: The 201 response body's `short_url` field is the alias prefixed with the service base URL.

## GET /:alias

- [x] **API-009**: When GET /:alias is called with an existing alias, it returns 301 with a `Location` header set to the original URL.
- [x] **API-010**: When GET /:alias is called with a non-existent alias, it returns 404.
- [ ] **API-011**: When GET /:alias is called with an alias that contains invalid characters, it returns 404 (not 400 — treat as not found).

## Error Responses

- [ ] **API-012**: All 400 error responses include a JSON body with an `error` field describing the validation failure.
- [ ] **API-013**: The 409 response includes a JSON body with an `error` field.
- [ ] **API-014**: 500 responses do not expose internal error details in the response body.
