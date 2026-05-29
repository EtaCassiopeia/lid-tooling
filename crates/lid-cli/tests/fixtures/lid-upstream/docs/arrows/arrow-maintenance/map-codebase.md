# Arrow: map-codebase

The `/map-codebase` skill — maps an existing codebase to a LID arrow structure. Produces `docs/arrows/index.yaml`, arrow docs, intent directories, and spec stubs from analysis of the codebase.

## Status

**MAPPED** — sampled 2026-04-25. Skill body committed; skill-creator iteration-1 run.

## References

### HLD
- `docs/high-level-design.md` § Architecture / Plugins (arrow-maintenance plugin)

### LLD
- `docs/intent/arrow-maintenance/arrow-maintenance-design.md` — shared plugin-level LLD covering both the audit skill and map-codebase.

### EARS
- `docs/intent/arrow-maintenance/map-codebase-specs.md` (prefix `MAP-CODEBASE-*`)

### Tests
- `plugins/arrow-maintenance/skills/map-codebase-workspace/iteration-1/`

### Code
- `plugins/arrow-maintenance/skills/map-codebase/SKILL.md`
- `plugins/arrow-maintenance/skills/map-codebase/references/`
- `plugins/arrow-maintenance/commands/map-codebase.md`
