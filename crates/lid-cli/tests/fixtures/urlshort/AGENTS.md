# URL Shortener

A URL shortener service used as the primary LID example project. Demonstrates the full schema v2 layout: parent/child segment hierarchy, multiple statuses, drift, next-work indicators, and rich spec coverage.

## LID
- Mode: Full
- Version: 1.3.0

## LID Directives

- Arrow overlay: `docs/arrows/index.yaml`
- HLD: `docs/high-level-design.md`
- Intent tree: `docs/intent/`
- Spec prefix pattern: path-derived (`{FOLDER}-{NNN}` per LID 1.2.0 convention)
  - `STORAGE-*` — storage interface contract
  - `STORAGE-STORE-INTERFACE-*` — store-interface (Store trait)
  - `STORAGE-IN-MEMORY-STORE-*` — in-memory-store
  - `STORAGE-REDIS-STORE-*` — redis-store
  - `SHORTENER-CORE-*` — shortener-core domain logic
  - `SHORTENER-CORE-ALIAS-GEN-*` — alias-gen
  - `SHORTENER-CORE-COLLISION-*` — collision handling
  - `API-*` — api HTTP surface
  - `API-SHORTEN-ENDPOINT-*` — shorten-endpoint (POST /shorten)
  - `API-REDIRECT-ENDPOINT-*` — redirect-endpoint (GET /:alias)
  - `API-HEALTH-ENDPOINT-*` — health-endpoint (GET /health)
  - `AUTH-*` — auth middleware
  - `RATE-LIMITER-*` — rate-limiter
  - `ANALYTICS-*` — analytics

### Memory vs. intent.
Before saving durable project knowledge to agent or tool memory, test whether it is project *intent* — would a fresh agent, in any tool, next session, need it to build this system correctly? If yes, record it in the arrow (HLD / LLD / EARS / decision doc), which travels and cascades — not in private, per-tool memory, where intent escapes the arrow. Knowledge about the user or how they like to work stays in memory.
