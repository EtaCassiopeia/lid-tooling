# Changelog

All notable changes to lid-tooling are documented here.
The version mirrors the upstream [LID](https://github.com/jszmajda/lid) release
it was built against. `PATCH` increments are tooling-only fixes.

## [1.2.0] — 2026-05-28

### Added

- Support for LID `schema_version: 2` (`docs/intent/` recursive tree layout,
  introduced in LID v1.2.0 / [jszmajda/lid#12](https://github.com/jszmajda/lid/pull/12))
- `SUPPORTED_SCHEMA_VERSIONS` constant in `lid-core` — explicit compatibility
  surface; hard error with migration hint on unsupported schema versions
- `load_md_tree` — recursive markdown walker for `docs/intent/` and nested
  `docs/arrows/` subdirectories

### Changed

- Minimum supported `schema_version`: **2** (schema v1 projects must migrate)
- Version bumped from 0.1.0 to 1.2.0 to mirror LID release cadence

### Supported schema versions

| `schema_version` | Layout | Status |
|-----------------|--------|--------|
| 1 | `docs/specs/` + `docs/llds/` (flat) | ❌ Not supported — migrate to v2 |
| 2 | `docs/intent/` recursive tree | ✅ Supported |
