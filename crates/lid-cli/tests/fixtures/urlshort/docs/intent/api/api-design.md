# LLD: api

**Created**: 2026-05-29
**Status**: Pending implementation

## Context

`api` is the HTTP surface for the URL shortener. It owns input validation at the transport boundary, maps HTTP semantics to `shortener-core` calls, and formats responses. It has no business logic of its own — all domain decisions belong to `shortener-core`.

## HLD Trace

- **Approach § api** — HTTP surface; input validation, error responses; depends on `shortener-core`.
- **Tenet / Interfaces at component boundaries** — `api` never calls `storage` directly.

## Endpoints

### POST /shorten

- Request body: `{ "url": "<string>" }`
- Success: `201 Created`, body `{ "alias": "<string>", "short_url": "<string>" }`
- Errors: `400` (validation), `409` (collision limit), `500` (unexpected)

### GET /:alias

- Success: `301 Moved Permanently`, `Location: <original-url>`
- Errors: `404` (not found), `500` (unexpected)

## Error Mapping

| shortener-core error | HTTP status |
|---|---|
| `InvalidUrl` | 400 |
| `UrlTooLong` | 400 |
| `CollisionLimit` | 409 |
| `ResolveError::NotFound` | 404 |

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Redirect status | 301 Permanent | 302 Temporary | Alias→URL mapping is immutable in v1; permanent redirect allows browser caching |
| Shorten response code | 201 Created | 200 OK | Semantically correct — a new resource was created |
