# Changelog

All notable changes to lid-tooling are documented here.
lid-tooling follows its own [semantic versioning](https://semver.org/) independent
of the upstream LID project. `PATCH` increments are tooling-only fixes.

## [0.3.0]

### Added

- `lidc init` and `lid_init` (MCP) now create `AGENTS.md` (primary instruction
  file) and `CLAUDE.md` (symlink alias on Unix; one-line `@AGENTS.md` import on
  Windows) when scaffolding a new project, tracking LID 1.3.0
  ([jszmajda/lid#28](https://github.com/jszmajda/lid/pull/28))
- `AGENTS.md` template includes the *Memory vs. intent* directive from LID 1.3.0:
  durable project knowledge is recorded in the arrow (HLD / LLD / EARS / decision
  doc) rather than in private per-tool agent memory
- Cursor is now a first-class LID plugin host — see upstream
  [LID 1.3.0 release notes](https://github.com/jszmajda/lid/releases/tag/v1.3.0)
  for setup details

### Changed

- Scaffold instruction-file template updated from LID 1.2.0 → 1.3.0

## [0.2.0]

### Added

- Support for LID `schema_version: 2` (`docs/intent/` recursive tree layout,
  introduced in [jszmajda/lid#12](https://github.com/jszmajda/lid/pull/12))
- `SUPPORTED_SCHEMA_VERSIONS` constant in `lid-core` — explicit compatibility
  surface; hard error with migration hint on unsupported schema versions
- `load_md_tree` — recursive markdown walker for `docs/intent/` and nested
  `docs/arrows/` subdirectories
- `lid-core::scaffold` module — `suggest_prefix`, `scaffold_intent_dir`, `scaffold_arrow_doc`
  pure helpers shared by all write surfaces
- `lidc init` now scaffolds a complete segment skeleton: `docs/arrows/<seg>/overview.md`
  (with `## References` bullets), `docs/intent/<seg>/<seg>-specs.md` (with `prefix:`
  frontmatter), and `docs/intent/<seg>/<seg>-design.md`; accepts `--spec-prefix` flag
- MCP `lid_init` and `lid_add_segment` scaffold the same intent files; both accept an
  optional `spec_prefix` field — omitting it triggers majority-namespace inference from
  existing prefixes (e.g. if most prefixes start with `USH-`, a new `billing` segment gets
  prefix `USH-BILLING`)
- VS Code and IntelliJ Intent Navigator "Add Segment" overlay: new **Spec Prefix** field,
  pre-filled by majority-namespace inference and updated live as the segment ID is typed;
  adding a segment now creates all three scaffold files
- `prefix:` YAML frontmatter in `*-specs.md` files: the `spec-id-format` check warns when a
  spec ID's prefix does not match the declared `prefix:` value

### Changed

- Minimum supported `schema_version`: **2** (schema v1 projects must migrate)

### Supported schema versions

| `schema_version` | Layout | Status |
|-----------------|--------|--------|
| 1 | `docs/specs/` + `docs/llds/` (flat) | ❌ Not supported — migrate to v2 |
| 2 | `docs/intent/` recursive tree | ✅ Supported |
