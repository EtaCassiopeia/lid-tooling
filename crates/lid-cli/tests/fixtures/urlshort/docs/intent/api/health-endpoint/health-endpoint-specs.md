---
prefix: API-HEALTH-ENDPOINT
---

# EARS Specs: health-endpoint

**Implementing artifacts**: `src/api.rs`, `tests/api_test.rs`

## Behaviour

- [x] **API-HEALTH-ENDPOINT-001**: When GET /health is called, it returns 200.
- [x] **API-HEALTH-ENDPOINT-002**: The response body is `{ "status": "ok" }` with `Content-Type: application/json`.
- [x] **API-HEALTH-ENDPOINT-003**: GET /health returns 200 even when the storage backend is unreachable (shallow probe).
