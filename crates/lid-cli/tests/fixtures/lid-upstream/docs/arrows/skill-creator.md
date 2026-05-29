# Arrow: skill-creator

The skill-creator segment owns the process and tooling for iteratively building, evaluating, and improving skill prompts (SKILL.md files). It drives the workspace-iteration loop that produces `lid-coach`, `arrow-maintenance`, and `bidirectional-differential` skills.

## Status

**MAPPED** — bootstrapped 2026-05-18. Core workflow specs implemented; eval-format and workspace-runner integration specs are open gaps.

## References

### HLD
- `docs/high-level-design.md` § Architecture / Plugins (skill-creator as a capability within arrow-maintenance)

### LLD
- `docs/intent/skill-creator/skill-creator-design.md`

### EARS
- `docs/intent/skill-creator/skill-creator-specs.md` (20 specs, prefix `SKILL-CREATOR-*`)

### Tests
- `plugins/arrow-maintenance/skills/map-codebase-workspace/iteration-1/` — map-codebase skill produced by skill-creator iteration-1
- `plugins/arrow-maintenance/skills/arrow-maintenance-workspace/iteration-1/` — arrow-maintenance skill produced by skill-creator iteration-1

### Code
- `plugins/arrow-maintenance/skills/map-codebase/SKILL.md` — the map-codebase skill (primary artifact of skill-creator's first production run)
- `plugins/arrow-maintenance/commands/map-codebase.md` — command stub

## Architecture

**Purpose:** Close the loop between LID's design artifacts (EARS specs) and the skills that implement them. Skill-creator is the mechanism by which LID skills are built and evaluated against their own specs — it consumes an EARS spec file, a reference SKILL.md, and an eval suite, then iterates toward a SKILL.md that satisfies the evals.

**Key Components:**
1. **Iteration workspace** (`{skill-name}-workspace/iteration-N/`) — outputs from each skill-creator run: the candidate SKILL.md, eval results, and a summary of changes.
2. **Eval format** — JSON eval suite consumed by skill-creator's runner; each entry has a prompt, expected assertions, and optional baseline comparison data.
3. **Promotion gate** — a skill is promoted from workspace output to canonical `skills/{name}/SKILL.md` when its iteration achieves target assertion coverage.

## Spec Coverage

| Category | Implemented | Active gap | Deferred |
|---|---|---|---|
| Invocation and dispatch (SKILL-CREATOR-001..005) | 5 `[x]` | 0 | 0 |
| Eval-format parsing (SKILL-CREATOR-006..011) | 6 `[x]` | 0 | 0 |
| Workspace runner (SKILL-CREATOR-012..017) | 0 | 6 `[ ]` | 0 |
| Promotion gate (SKILL-CREATOR-018..020) | 3 `[x]` | 0 | 0 |
| **Total** | **14** | **6** | **0** |

**Summary:** 14 of 20 specs implemented. The 6 open gaps are the workspace-runner wiring specs — the runner exists but is not yet connected to the eval format from the workspace iteration step.

## Work Required

### Must Fix
1. **SKILL-CREATOR-012 through SKILL-CREATOR-017** — wire workspace runner to eval format so iteration outputs include structured eval results alongside the candidate SKILL.md.

### Nice to Have
2. Add iteration-2 workspaces for `lid-coach` and `arrow-maintenance` once SKILL-CREATOR-012..017 are closed.
