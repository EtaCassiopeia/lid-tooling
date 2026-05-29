# skill-creator specs

**LLD**: docs/intent/skill-creator/skill-creator-design.md
**Implementing artifacts**:
- plugins/arrow-maintenance/skills/map-codebase/SKILL.md

Status markers: `[x]` implemented · `[ ]` active gap · `[D]` deferred

---

## Invocation

- `[x]` **SKILL-CREATOR-001**: When the user invokes `/map-codebase`, the system SHALL dispatch to the `map-codebase` skill.
- `[x]` **SKILL-CREATOR-002**: The skill SHALL accept a target skill name, a path to an EARS spec file, and an optional reference SKILL.md as inputs.
- `[x]` **SKILL-CREATOR-003**: When no reference SKILL.md is provided, the skill SHALL generate an initial SKILL.md from the EARS spec file alone.
- `[x]` **SKILL-CREATOR-004**: The skill SHALL create a workspace directory at `{skill-name}-workspace/iteration-{N}/` where N is the next iteration number.
- `[x]` **SKILL-CREATOR-005**: The skill SHALL write the candidate SKILL.md, eval results summary, and a diff from the previous iteration into the workspace directory.

## Eval-Format Parsing

- `[x]` **SKILL-CREATOR-006**: The eval suite SHALL be a JSON file with an `evals` array; each entry SHALL have `id`, `prompt`, and `assertions` fields.
- `[x]` **SKILL-CREATOR-007**: Each assertion SHALL have a `type` field (one of: `contains`, `not_contains`, `regex`) and a `value` field.
- `[x]` **SKILL-CREATOR-008**: The skill SHALL report the fraction of assertions satisfied as `{passed}/{total}` in the iteration summary.
- `[x]` **SKILL-CREATOR-009**: The skill SHALL distinguish between with-skill assertions (run against the candidate SKILL.md) and baseline assertions (run without any skill).
- `[x]` **SKILL-CREATOR-010**: The skill SHALL surface non-discriminating assertions (baseline already passes) as candidates to strengthen in the next iteration.
- `[x]` **SKILL-CREATOR-011**: The skill SHALL include per-eval assertion results in the workspace output so failures can be inspected individually.

## Workspace Runner

- `[ ]` **SKILL-CREATOR-012**: The workspace runner SHALL be invocable from the skill-creator command without requiring a separate manual step.
- `[ ]` **SKILL-CREATOR-013**: The runner SHALL execute each eval entry by calling `claude -p` with the candidate SKILL.md injected as system context.
- `[ ]` **SKILL-CREATOR-014**: The runner SHALL execute baseline runs (without the skill) for the same prompts to compute discriminating-assertion coverage.
- `[ ]` **SKILL-CREATOR-015**: The runner SHALL parallelize eval execution up to a configurable concurrency limit (default: 3).
- `[ ]` **SKILL-CREATOR-016**: The runner SHALL produce a structured JSON result file in the workspace directory alongside the human-readable summary.
- `[ ]` **SKILL-CREATOR-017**: The runner SHALL detect and report eval entries where the assertion type is unsupported, rather than silently treating them as passing.

## Promotion Gate

- `[x]` **SKILL-CREATOR-018**: A skill SHALL be considered promotion-ready when its latest iteration achieves ≥ 90% with-skill assertion coverage across all non-deferred assertions.
- `[x]` **SKILL-CREATOR-019**: The promotion gate SHALL be documented in the iteration summary so the decision to promote is auditable.
- `[x]` **SKILL-CREATOR-020**: When a skill is promoted, the workspace directory SHALL be preserved as the historical record; the canonical SKILL.md is updated at its stable path.
