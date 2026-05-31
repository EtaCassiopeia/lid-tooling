---
title: Project Layout
nav_order: 7
---

# Project Layout (schema v2)

{: .no_toc }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Directory structure

```
docs/
├── arrows/
│   ├── index.yaml              # segment graph — statuses, blocks, taxonomy
│   └── <segment>/
│       └── *.md                # arrow detail docs (HLDs, LLDs)
└── intent/
    └── <segment>/
        ├── <segment>-specs.md  # spec lines  (- [ ] **ID**: text)
        └── <segment>-design.md # design docs
```

---

## `index.yaml`

Minimal shape:

```yaml
schema_version: 2
arrows:
  auth:
    status: MAPPED
    detail: auth/core.md
```

Full example with dependencies and taxonomy:

```yaml
schema_version: 2
arrows:
  auth:
    status: OK
    detail: auth/core.md
    blocks:
      - api
  api:
    status: MAPPED
    detail: api/core.md
    parent: platform
  platform:
    status: AUDITED
    detail: platform/core.md
    children:
      - api
```

### Segment statuses

| Status | Meaning |
|--------|---------|
| `UNMAPPED` | Intent not yet described |
| `MAPPED` | Intent described, not yet audited |
| `AUDITED` | Intent audited against implementation |
| `OK` | Fully implemented and verified |
| `MERGED` | Absorbed into another segment |

---

## Spec line format

Spec lines live in `docs/intent/<segment>/<segment>-specs.md`:

```markdown
- [ ] **AUTH-001**: the system shall authenticate users via password.
- [x] **AUTH-002**: sessions shall expire after 30 minutes of inactivity.
- [D] **AUTH-003**: biometric login is deferred to v2.
```

| Marker | Status |
|--------|--------|
| `[ ]` | Open — not yet implemented |
| `[x]` | Implemented — must have at least one `@spec` citation |
| `[D]` | Deferred — must not have `@spec` citations |

### Spec ID format

`<SEGMENT>-<NNN>` where `<SEGMENT>` is the uppercase segment name and `<NNN>` is a zero-padded number. Example: `AUTH-001`, `PAYMENTS-042`.

---

## Source citation format

Cite a spec from any language using a line comment:

```rust
// @spec AUTH-002
fn validate_session_expiry(session: &Session) -> bool { ... }
```

```typescript
// @spec AUTH-001
export async function authenticate(credentials: Credentials) { ... }
```

```python
# @spec AUTH-001
def authenticate(credentials: dict) -> User:
```

```go
// @spec AUTH-002
func validateSessionExpiry(session *Session) bool {
```

The `@spec` keyword is case-sensitive and must be followed by the spec ID. Multiple citations on adjacent lines are allowed.

---

## Schema versioning

| `schema_version` | Layout | Status |
|-----------------|--------|--------|
| 1 | `docs/specs/` + `docs/llds/` (flat) | ❌ Not supported — migrate to v2 |
| 2 | `docs/intent/` recursive tree | ✅ Supported |

Encountering an unsupported schema version produces a hard error with a migration hint — never a silent partial load.
