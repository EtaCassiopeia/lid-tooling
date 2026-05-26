# Sample LID Project

A minimal LID-shaped project for exercising `lidc` and `lid-lsp`
end to end. The domain is a toy authentication flow — five EARS
specs across login / logout / lockout / password-reset, three of
them implemented with `@spec` citations, two intentionally left
as gaps.

## Layout

```
docs/
├── arrows/
│   ├── index.yaml       # the arrow overlay (the project's index)
│   ├── auth.md          # per-segment detail for the `auth` arrow
│   └── session.md       # placeholder for the `session` arrow
├── specs/
│   └── auth-specs.md    # AUTH-001 … AUTH-005
└── llds/
    └── auth.md          # design decisions for the auth segment

src/
├── login.ts             # cites AUTH-001, AUTH-002
├── logout.ts            # cites AUTH-003
└── session.ts           # stubs; no spec yet

tests/
├── login.test.ts        # cites AUTH-001, AUTH-002
└── logout.test.ts       # cites AUTH-003
```

## Expected state

`lidc check` against this directory should report zero findings.
Use it as a baseline; the walkthrough in the top-level README
introduces deliberate issues to demonstrate diagnostics.
