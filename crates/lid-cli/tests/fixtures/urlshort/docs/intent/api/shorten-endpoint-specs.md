---
prefix: USH-POST
---

# EARS Specs: shorten-endpoint

**Implementing artifacts**: `src/api.rs`, `tests/api_test.rs`

## Happy Path

- [x] **USH-POST-001**: When POST /shorten is called with a valid URL and no existing alias, it returns 201 with `{ "alias": "...", "short_url": "..." }`.
- [x] **USH-POST-002**: When POST /shorten is called with a URL that already has an alias, it returns 200 with the same alias and `short_url`.
- [x] **USH-POST-003**: The `short_url` field in the response is the base URL of the service concatenated with the alias.

## Error Cases

- [ ] **USH-POST-004**: When POST /shorten results in `ShortenError::CollisionLimit`, it returns 409 with `{ "error": "alias collision limit exceeded" }`.
- [ ] **USH-POST-005**: When POST /shorten is called without a valid `Authorization` header, it returns 401.
