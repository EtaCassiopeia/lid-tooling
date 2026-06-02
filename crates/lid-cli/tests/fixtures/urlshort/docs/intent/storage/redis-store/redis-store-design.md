# LLD: redis-store

**Created**: 2026-05-29  
**Status**: Draft — pending infrastructure

## Context

`RedisStore` replaces `InMemoryStore` in production once the Redis cluster is provisioned. It satisfies the same `Store` trait, so no changes are needed to `shortener-core` or `api`.

## Key Schema

```
SET ush:{alias} {url}  [no TTL — aliases are permanent]
GET ush:{alias}        → url or nil
```

Aliases are permanent by design; no TTL is set. A future `analytics` component may read this key space for redirect tracking.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Key prefix | `ush:` | none, `alias:` | Namespaces keys so the Redis instance can be shared with other services |
| TTL | None (permanent) | Per-alias TTL | Aliases are immutable; expiry is not a v1 requirement |
| Connection pooling | `deadpool-redis` | `redis-rs` direct | Pooling avoids per-request connection overhead under load |
