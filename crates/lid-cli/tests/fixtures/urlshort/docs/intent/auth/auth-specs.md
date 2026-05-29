# EARS Specs: auth

**Implementing artifacts**: `src/auth.rs`, `tests/auth_test.rs`

## Valid Requests

- [ ] **USH-AUTH-001**: When a request carries a valid `Authorization: Bearer {key_id}:{hmac}` header, the middleware forwards the request to the next handler.
- [ ] **USH-AUTH-002**: The HMAC is valid when it equals `HMAC-SHA256(secret_for_key_id, key_id + ":" + today_utc_date)`.

## Rejection Cases

- [ ] **USH-AUTH-003**: When the `Authorization` header is absent, the middleware returns 401 with `{ "error": "missing authorization header" }`.
- [ ] **USH-AUTH-004**: When the `Authorization` header is present but malformed, the middleware returns 401 with `{ "error": "invalid authorization format" }`.
- [ ] **USH-AUTH-005**: When the `key_id` is not in the configured key set, the middleware returns 401 with `{ "error": "unknown api key" }`.
- [ ] **USH-AUTH-006**: When the HMAC does not match the expected value, the middleware returns 401 with `{ "error": "invalid api key" }`.
