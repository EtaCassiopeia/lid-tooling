# Storage Backend: Redis as Primary, In-Memory for Tests

Use Redis as the production URL-store and an in-memory implementation for local
development and integration tests.

## Context

We need a URL store that survives process restarts in production and allows
multiple service instances to share state behind a load balancer. For local
development and CI we want zero external dependencies.

## Decision

Provide two implementations of the `Store` trait:
- `InMemoryStore` — default for tests and `cargo run` without config
- `RedisStore` — selected at startup via the `REDIS_URL` environment variable

The `Store` trait is defined in `storage/store-interface-specs.md`; both
implementations must satisfy all `USH-SITF-*` specs.

## Consequences

- Adding a third backend (e.g., PostgreSQL) only requires a new `Store` impl;
  no changes to `shortener-core` or the API layer.
- The in-memory store is not shared across processes — acceptable for dev/test,
  rejected for production.
- Redis dependency is optional at compile time; feature-flag it if the binary
  size becomes a concern.
