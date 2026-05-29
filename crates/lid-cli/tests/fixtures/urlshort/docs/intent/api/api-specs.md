# EARS Specs: api

**Implementing artifacts**: `src/api.rs`, `tests/api_test.rs`

## POST /shorten

- [ ] USH-API-001: When POST /shorten is called with a valid URL, it returns 201 with `{ "alias": "...", "short_url": "..." }`.
- [ ] USH-API-002: When POST /shorten is called with a URL that already has an alias, it returns 200 with the existing alias (idempotent re-submission).
- [ ] USH-API-003: When POST /shorten is called with a non-HTTP/HTTPS URL, it returns 400.
- [ ] USH-API-004: When POST /shorten is called with a URL exceeding 2048 characters, it returns 400.
- [ ] USH-API-005: When POST /shorten is called with a malformed JSON body, it returns 400.
- [ ] USH-API-006: When POST /shorten is called with a missing `url` field, it returns 400.
- [ ] USH-API-007: When POST /shorten results in a collision limit error, it returns 409.
- [ ] USH-API-008: The 201 response body's `short_url` field is the alias prefixed with the service base URL.

## GET /:alias

- [ ] USH-API-009: When GET /:alias is called with an existing alias, it returns 301 with a `Location` header set to the original URL.
- [ ] USH-API-010: When GET /:alias is called with a non-existent alias, it returns 404.
- [ ] USH-API-011: When GET /:alias is called with an alias that contains invalid characters, it returns 404 (not 400 — treat as not found).

## Error Responses

- [ ] USH-API-012: All 400 error responses include a JSON body with an `error` field describing the validation failure.
- [ ] USH-API-013: The 409 response includes a JSON body with an `error` field.
- [ ] USH-API-014: 500 responses do not expose internal error details in the response body.
