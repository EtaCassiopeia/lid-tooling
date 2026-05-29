# LLD: alias-gen

**Created**: 2026-04-20  
**Status**: Implemented and audited

## Context

`alias-gen` converts a long URL into a 7-character base62 alias. The algorithm must be deterministic (same input → same output) and produce unique-enough outputs that collisions are rare at projected scale.

## Algorithm

1. Compute SHA-256 of the URL string (UTF-8 encoded).
2. Take the first 6 bytes of the digest → 48 bits.
3. Encode as base62 (digits 0–9, then A–Z, then a–z) → 7 characters (62^7 ≈ 3.5 trillion).

The determinism property is what makes idempotency free — no lookup required to decide whether to reuse an existing alias.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Hash function | SHA-256 first 6 bytes | MD5, xxHash, random | Cryptographically strong, universally available; first 6 bytes gives enough bits |
| Output alphabet | base62 (0-9A-Za-z) | base64, hex | URL-safe without percent-encoding; familiar to users |
| Output length | 7 characters | 6, 8 | 7 base62 chars = 62^7 ≈ 3.5T; safe for years of growth |
