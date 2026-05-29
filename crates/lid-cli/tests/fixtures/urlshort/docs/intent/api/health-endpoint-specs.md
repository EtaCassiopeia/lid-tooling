# EARS Specs: health-endpoint

**Implementing artifacts**: `src/api.rs`, `tests/api_test.rs`

## Behaviour

- [x] **USH-HLTH-001**: When GET /health is called, it returns 200.
- [x] **USH-HLTH-002**: The response body is `{ "status": "ok" }` with `Content-Type: application/json`.
- [x] **USH-HLTH-003**: GET /health returns 200 even when the storage backend is unreachable (shallow probe).
