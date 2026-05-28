# LLD: auth

## Context

Handles user authentication — login, logout, session tokens — and
the eventual lockout / password-reset flows. Session storage is a
separate segment (`session`), so this LLD assumes a `SessionStore`
interface exists and doesn't specify its internals.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| Session-token format | Opaque random (32 bytes, base64url) | JWT, signed envelope | Avoid leaking claims via decodable tokens; rotation is cheap |
| Password hashing | argon2id | bcrypt, scrypt | Modern, memory-hard, tuned for current hardware |
| Lockout state | Redis counter keyed by `username` | DB row, in-memory | Per-pod state would let attackers retry across pods |
| Reset-token TTL | 30 minutes | 24 hours, 5 minutes | Short enough to limit damage, long enough for email delay |

## Open Questions

- Should successful login reset the lockout counter, or only the
  per-IP counter? Current plan: both, but pending discussion.
