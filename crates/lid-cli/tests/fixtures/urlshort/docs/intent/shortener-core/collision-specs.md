# EARS Specs: collision

**Implementing artifacts**: `src/shortener_core.rs`, `tests/shortener_core_test.rs`

## Retry Logic

- [x] **USH-COLL-001**: When the base alias is free, `shorten` stores it and returns it without any suffix.
- [ ] **USH-COLL-002**: When the base alias is taken by a different URL, `shorten` tries `alias_1` before returning an error.
- [ ] **USH-COLL-003**: When `alias_1` through `alias_4` are all taken by different URLs, `shorten` tries `alias_5` as the final candidate.
- [ ] **USH-COLL-004**: When `alias_1` through `alias_5` are all taken by different URLs, `shorten` returns `ShortenError::CollisionLimit`.
- [ ] **USH-COLL-005**: When any candidate alias is taken by the *same* URL as the input, `shorten` returns that alias immediately without writing.
