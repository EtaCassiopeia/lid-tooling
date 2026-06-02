# LLD: shorten-endpoint

**Created**: 2026-05-10  
**Status**: Partial — happy path and idempotency shipped

## Context

`POST /shorten` is the primary write endpoint. It parses a JSON body, delegates to `shortener-core`, and maps domain errors to HTTP status codes.

## Request / Response

```
POST /shorten
Content-Type: application/json
Authorization: Bearer <api-key>

{ "url": "https://example.com/very/long/path" }

201 Created
{ "alias": "aB3dEfG", "short_url": "https://s.example.com/aB3dEfG" }

200 OK  (idempotent re-submission)
{ "alias": "aB3dEfG", "short_url": "https://s.example.com/aB3dEfG" }
```

## Error Mapping

| Domain error | HTTP status |
|---|---|
| `InvalidUrl` | 400 |
| `UrlTooLong` | 400 |
| `CollisionLimit` | 409 |
| Unexpected storage error | 500 (no details exposed) |

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| New vs existing alias status code | 201 / 200 | Always 201 or always 200 | Distinguishable by client; 201 signals resource creation |
| `short_url` field | Include in response | Omit (alias only) | Saves clients a string concatenation; negligible overhead |
