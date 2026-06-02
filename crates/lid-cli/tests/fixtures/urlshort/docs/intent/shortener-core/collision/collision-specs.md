---
prefix: SHORTENER-CORE-COLLISION
---

# EARS Specs: collision

**Implementing artifacts**: `src/shortener_core.rs`, `tests/shortener_core_test.rs`

## Retry Logic

- [x] **SHORTENER-CORE-COLLISION-001**: When the base alias is free, `shorten` stores it and returns it without any suffix.
- [ ] **SHORTENER-CORE-COLLISION-002**: When the base alias is taken by a different URL, `shorten` tries `alias_1` before returning an error.
- [ ] **SHORTENER-CORE-COLLISION-003**: When `alias_1` through `alias_4` are all taken by different URLs, `shorten` tries `alias_5` as the final candidate.
- [ ] **SHORTENER-CORE-COLLISION-004**: When `alias_1` through `alias_5` are all taken by different URLs, `shorten` returns `ShortenError::CollisionLimit`.
- [ ] **SHORTENER-CORE-COLLISION-005**: When any candidate alias is taken by the *same* URL as the input, `shorten` returns that alias immediately without writing.
