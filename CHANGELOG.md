# Changelog

All notable changes to lid-tooling are documented here.
lid-tooling follows its own [semantic versioning](https://semver.org/) independent
of the upstream LID project. `PATCH` increments are tooling-only fixes.

## [Unreleased]

### Added

- `lid-core::scaffold` module — `suggest_prefix`, `scaffold_intent_dir`, `scaffold_arrow_doc`
  pure helpers shared by all write surfaces
- `lidc init` now scaffolds a complete segment skeleton: `docs/arrows/<seg>/overview.md`
  (with `## References` bullets), `docs/intent/<seg>/<seg>-specs.md` (with `prefix:`
  frontmatter), and `docs/intent/<seg>/<seg>-design.md`; accepts `--spec-prefix` flag
- MCP `lid_init` and `lid_add_segment` now scaffold the same intent files; both accept an
  optional `spec_prefix` field — omitting it triggers majority-namespace inference from
  existing prefixes (e.g. if most prefixes start with `USH-`, a new `billing` segment gets
  prefix `USH-BILLING`)
- VS Code and IntelliJ Intent Navigator "Add Segment" overlay: new **Spec Prefix** field,
  pre-filled by majority-namespace inference and updated live as the segment ID is typed;
  adding a segment now creates all three scaffold files
- `prefix:` YAML frontmatter in `*-specs.md` files: the `spec-id-format` check warns when a
  spec ID's prefix does not match the declared `prefix:` value

---

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
