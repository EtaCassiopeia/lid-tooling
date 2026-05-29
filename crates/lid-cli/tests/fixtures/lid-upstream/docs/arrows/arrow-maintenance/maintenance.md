# Arrow: maintenance

The `/arrow-maintenance` skill — deterministic structural audit for LID projects. Walks the arrow overlay, detects drift (spec coverage, reference coherence, orphans, schema issues, DAG cycles), and produces a structured findings report.

## Status

**MAPPED** — sampled 2026-04-25. Skill body and reference materials committed; first audit run completed 2026-04-25.

## References

### HLD
- `docs/high-level-design.md` § Architecture / Plugins (arrow-maintenance plugin); § Key Design Decisions / The arrow for LID itself

### LLD
- `docs/intent/arrow-maintenance/arrow-maintenance-design.md` — covers the audit skill, checklist, and plugin-level concerns.

### EARS
- `docs/intent/arrow-maintenance/arrow-maintenance-specs.md` (prefix `ARROW-MAINT-*`)

### Tests
- `plugins/arrow-maintenance/skills/arrow-maintenance-workspace/iteration-1/`

### Code
- `plugins/arrow-maintenance/skills/arrow-maintenance/SKILL.md`
- `plugins/arrow-maintenance/skills/arrow-maintenance/references/`
- `plugins/arrow-maintenance/commands/arrow-maintenance.md`
