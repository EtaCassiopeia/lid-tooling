# Changelog

All notable changes to lid-tooling are documented here.
lid-tooling follows its own [semantic versioning](https://semver.org/) independent
of the upstream LID project. `PATCH` increments are tooling-only fixes.

## [0.2.0] — 2026-05-28

### Added

- Support for LID `schema_version: 2` (`docs/intent/` recursive tree layout,
  introduced in [jszmajda/lid#12](https://github.com/jszmajda/lid/pull/12))
- `SUPPORTED_SCHEMA_VERSIONS` constant in `lid-core` — explicit compatibility
  surface; hard error with migration hint on unsupported schema versions
- `load_md_tree` — recursive markdown walker for `docs/intent/` and nested
  `docs/arrows/` subdirectories

### Changed

- Minimum supported `schema_version`: **2** (schema v1 projects must migrate)

### Supported schema versions

| `schema_version` | Layout | Status |
|-----------------|--------|--------|
| 1 | `docs/specs/` + `docs/llds/` (flat) | ❌ Not supported — migrate to v2 |
| 2 | `docs/intent/` recursive tree | ✅ Supported |
