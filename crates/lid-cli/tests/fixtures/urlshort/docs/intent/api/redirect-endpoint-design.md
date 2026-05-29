# LLD: redirect-endpoint

**Created**: 2026-05-10  
**Status**: Partial — redirect and 404 implemented

## Context

`GET /:alias` resolves an alias to a URL and redirects the caller. This is the hot path; latency is the primary concern.

## Response

```
GET /aB3dEfG

301 Moved Permanently
Location: https://example.com/very/long/path

404 Not Found   (unknown or malformed alias)
```

## Design Notes

- 301 (permanent) is intentional — aliases are immutable. Enables aggressive browser and CDN caching.
- Malformed aliases (invalid characters) are treated as 404, not 400. The alias namespace owns what's valid; returning 400 leaks internal format details.
- No authentication required. Redirect is a public read operation.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Redirect code | 301 Permanent | 302 Temporary | Aliases never change; permanent redirect enables CDN caching |
| Malformed alias response | 404 | 400 | Avoids leaking alias format; consistent with unknown-alias path |
