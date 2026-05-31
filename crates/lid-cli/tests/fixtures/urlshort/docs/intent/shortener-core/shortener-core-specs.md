---
prefix: USH-CORE
---

# EARS Specs: shortener-core

**Implementing artifacts**: `src/shortener_core.rs`, `tests/shortener_core_test.rs`

## Alias Generation

- [x] **USH-CORE-001**: When `shorten` is called with a valid HTTP/HTTPS URL, it returns a 7-character base62 alias.
- [x] **USH-CORE-002**: When `shorten` is called with the same URL twice, it returns the same alias both times (idempotency).
- [x] **USH-CORE-003**: When the computed alias is already taken by a different URL, `shorten` appends `_1` and retries.
- [x] **USH-CORE-004**: When all five collision candidates are taken by different URLs, `shorten` returns `ShortenError::CollisionLimit`.
- [ ] **USH-CORE-005**: When the computed alias is already taken by the same URL, `shorten` returns the existing alias without writing.

## Validation

- [ ] **USH-CORE-006**: When `shorten` is called with a non-HTTP/HTTPS URL, it returns `ShortenError::InvalidUrl`.
- [ ] **USH-CORE-007**: When `shorten` is called with a URL longer than 2048 characters, it returns `ShortenError::UrlTooLong`.
- [ ] **USH-CORE-008**: When `shorten` is called with a URL containing control characters (0x00–0x1F, 0x7F), it returns `ShortenError::InvalidUrl`.

## Resolution

- [x] **USH-CORE-009**: When `resolve` is called with an existing alias, it returns the original URL.
- [ ] **USH-CORE-010**: When `resolve` is called with an alias that does not exist, it returns `ResolveError::NotFound`.

## Storage Interaction

- [ ] **USH-CORE-011**: `shorten` calls `storage.get(alias)` before `storage.put(alias, url)` to check for collisions.
- [ ] **USH-CORE-012**: `shorten` calls `storage.put` at most once per invocation (no redundant writes on idempotent re-submissions).
- [ ] **USH-CORE-013**: `resolve` calls `storage.get` exactly once per invocation.
