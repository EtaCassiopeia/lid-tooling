---
prefix: SHORTENER-CORE-ALIAS-GEN
---

# EARS Specs: alias-gen

**Implementing artifacts**: `src/shortener_core.rs`, `tests/shortener_core_test.rs`

## Determinism

- [x] **SHORTENER-CORE-ALIAS-GEN-001**: When `generate_alias` is called with the same URL twice, it returns the same alias both times.
- [x] **SHORTENER-CORE-ALIAS-GEN-002**: When `generate_alias` is called with two different URLs, the probability of returning the same alias is less than 1 in 3.5 trillion.

## Output Format

- [x] **SHORTENER-CORE-ALIAS-GEN-003**: `generate_alias` always returns exactly 7 characters.
- [x] **SHORTENER-CORE-ALIAS-GEN-004**: Every character in the returned alias is in the set `[0-9A-Za-z]`.

## Edge Cases

- [x] **SHORTENER-CORE-ALIAS-GEN-005**: When `generate_alias` is called with a URL that contains Unicode characters, it returns a valid 7-character alias without error.
- [x] **SHORTENER-CORE-ALIAS-GEN-006**: When `generate_alias` is called with the minimum valid URL (`http://a.b`), it returns a valid alias.
