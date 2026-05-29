# EARS Specs: alias-gen

**Implementing artifacts**: `src/shortener_core.rs`, `tests/shortener_core_test.rs`

## Determinism

- [x] **USH-ALIAS-001**: When `generate_alias` is called with the same URL twice, it returns the same alias both times.
- [x] **USH-ALIAS-002**: When `generate_alias` is called with two different URLs, the probability of returning the same alias is less than 1 in 3.5 trillion.

## Output Format

- [x] **USH-ALIAS-003**: `generate_alias` always returns exactly 7 characters.
- [x] **USH-ALIAS-004**: Every character in the returned alias is in the set `[0-9A-Za-z]`.

## Edge Cases

- [x] **USH-ALIAS-005**: When `generate_alias` is called with a URL that contains Unicode characters, it returns a valid 7-character alias without error.
- [x] **USH-ALIAS-006**: When `generate_alias` is called with the minimum valid URL (`http://a.b`), it returns a valid alias.
