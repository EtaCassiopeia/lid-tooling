# High-Level Design: URL Shortener

A minimal URL shortener service. Given a long URL it returns a short alias; given that alias it redirects to the original URL. The design is stack-agnostic — the three components below are logical boundaries, not implementation choices.

## Goals

1. **Shorten URLs reliably.** Alias generation is deterministic for the same long URL and idempotent on re-submission.
2. **Redirect fast.** Lookup latency is the user-facing metric; alias storage must support O(1) reads.
3. **Stay a minimum system.** No analytics, no custom aliases, no expiry in v1. Each addition must justify itself against this goal.

## Approach

Three components with clear interface boundaries:

1. **shortener-core** — the domain logic: alias generation algorithm, collision handling, and the core business rules. Stateless; depends on `storage` for persistence.
2. **api** — HTTP surface: endpoints for shorten and redirect, input validation, error responses. Depends on `shortener-core`.
3. **storage** — persistence abstraction: CRUD operations over alias→URL pairs. The interface is owned here; the implementation (in-memory, Redis, Postgres) is injected at startup.

## Tenets

- **Simplicity over flexibility.** A new feature request that requires a new component is a red flag — absorb into existing components first.
- **Interfaces at component boundaries.** `api` never calls `storage` directly; `shortener-core` is the only mediator.
- **Tests before code.** Each component's EARS specs are the contract; code that passes the specs is done.

## Key Design Decisions

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Alias length | 7 chars (base62) | shorter/longer | ~3.5T aliases before collision pressure; short enough for readability |
| Collision strategy | Append counter, retry up to 5× | random regeneration | Deterministic, bounded, testable |
| Storage interface ownership | `storage` component owns the interface | `shortener-core` owns it | Keeps storage swappable without touching domain logic |
