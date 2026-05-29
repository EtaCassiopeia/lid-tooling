# Arrow: update-lid

The behavioral `update-lid` skill — bootstrap and idempotent reconciliation for LID project configuration. Invokable as `/update-lid`; also a sub-step from `/linked-intent-dev` Phase 1 and `/map-codebase`'s terminal step.

## Status

**MAPPED** — all 34 UPDATE-LID specs implemented and marked `[x]`.

## References

### HLD
- `docs/high-level-design.md` § Architecture / Plugins (linked-intent-dev plugin); § Key Design Decisions / The arrow for LID itself

### LLD
- `docs/intent/linked-intent-dev/linked-intent-dev-design.md` — plugin-level LLD covering mode detection, spec ID format, LID-on-LID linkage inversion, and eval metadata schema.

### EARS
- `docs/intent/linked-intent-dev/update-lid-specs.md` (34 specs, prefix `UPDATE-LID-*`)

### Tests
- `plugins/linked-intent-dev/skills/update-lid-workspace/` (skill-creator iteration outputs; latest: iteration-1, three evals)

### Code
- `plugins/linked-intent-dev/skills/update-lid/SKILL.md` + `plugins/linked-intent-dev/skills/update-lid/references/`

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All UPDATE-LID | UPDATE-LID-001..034 | 34 | 0 | 0 |

**Summary:** All 34 specs implemented; `update-lid` is feature-complete.
