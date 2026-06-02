# LLD: health-endpoint

**Created**: 2026-05-10  
**Status**: Implemented and audited

## Context

`GET /health` is a shallow liveness probe. It returns 200 with a fixed JSON body — no dependency checks, no database pings. Kubernetes and load-balancer health checks hit this endpoint.

## Response

```
GET /health

200 OK
Content-Type: application/json
{ "status": "ok" }
```

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Depth | Shallow (no dep checks) | Deep (ping storage) | A deep check makes the service appear unhealthy when storage is slow, causing unnecessary restarts |
| Response body | `{ "status": "ok" }` | Plain text `ok`, empty body | JSON is consistent with other endpoints; parseable by monitoring tooling |
