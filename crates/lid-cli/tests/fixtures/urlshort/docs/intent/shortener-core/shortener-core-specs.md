---
prefix: SHORTENER-CORE
---

# EARS Specs: shortener-core

**Implementing artifacts**: `src/shortener_core.rs`, `tests/shortener_core_test.rs`

## Alias Generation

- [x] **SHORTENER-CORE-001**: When `shorten` is called with a valid HTTP/HTTPS URL, it returns a 7-character base62 alias.
- [x] **SHORTENER-CORE-002**: When `shorten` is called with the same URL twice, it returns the same alias both times (idempotency).
- [x] **SHORTENER-CORE-003**: When the computed alias is already taken by a different URL, `shorten` appends `_1` and retries.
- [x] **SHORTENER-CORE-004**: When all five collision candidates are taken by different URLs, `shorten` returns `ShortenError::CollisionLimit`.
- [ ] **SHORTENER-CORE-005**: When the computed alias is already taken by the same URL, `shorten` returns the existing alias without writing.

## Validation

- [ ] **SHORTENER-CORE-006**: When `shorten` is called with a non-HTTP/HTTPS URL, it returns `ShortenError::InvalidUrl`.
- [ ] **SHORTENER-CORE-007**: When `shorten` is called with a URL longer than 2048 characters, it returns `ShortenError::UrlTooLong`.
- [ ] **SHORTENER-CORE-008**: When `shorten` is called with a URL containing control characters (0x00–0x1F, 0x7F), it returns `ShortenError::InvalidUrl`.

## Resolution

- [x] **SHORTENER-CORE-009**: When `resolve` is called with an existing alias, it returns the original URL.
- [ ] **SHORTENER-CORE-010**: When `resolve` is called with an alias that does not exist, it returns `ResolveError::NotFound`.

## Storage Interaction

- [ ] **SHORTENER-CORE-011**: `shorten` calls `storage.get(alias)` before `storage.put(alias, url)` to check for collisions.
- [ ] **SHORTENER-CORE-012**: `shorten` calls `storage.put` at most once per invocation (no redundant writes on idempotent re-submissions).
- [ ] **SHORTENER-CORE-013**: `resolve` calls `storage.get` exactly once per invocation.
