# LLD: analytics

**Created**: 2026-05-29  
**Status**: Draft — storage backend not yet chosen

## Context

`analytics` will capture per-alias redirect events and expose aggregated click counts. It is explicitly out of scope for v1 but included in the design to ensure the v1 architecture doesn't prevent it from being added cleanly.

## Proposed Architecture

The redirect endpoint emits an event on each successful 301. An `analytics` sidecar or background worker consumes these events and writes to a time-series store. The redirect latency must not be affected (fire-and-forget via a bounded channel).

## Storage Options Under Evaluation

| Option | Pros | Cons |
|---|---|---|
| ClickHouse | Excellent compression, fast aggregations | Operational overhead |
| TimescaleDB | Postgres-compatible, familiar tooling | Less efficient for pure append workloads |
| Redis sorted sets | Zero new infrastructure | Not durable; limited query flexibility |

Decision to be made before EARS are authored.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Event delivery | Fire-and-forget channel | Synchronous, persistent queue | Redirect latency must not increase; analytics can tolerate occasional loss |
| Storage backend | TBD | ClickHouse, TimescaleDB, Redis | Evaluation in progress; HLD addendum required |
