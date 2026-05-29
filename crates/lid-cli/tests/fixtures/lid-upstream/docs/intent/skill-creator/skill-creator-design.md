# LLD: skill-creator

**Created**: 2026-05-18
**Status**: Active — iteration-1 complete for map-codebase and arrow-maintenance skills

## Context and Design Philosophy

The skill-creator process closes the loop between EARS specs and working Claude skills. It is not a separate plugin but a capability owned by the `arrow-maintenance` segment: the `map-codebase` skill is the implementation surface, and `skill-creator` is the segment-level framing for that surface's iterative-development story.

The core insight: a SKILL.md is itself an artifact with EARS specs, and building it should follow the same workflow as any other LID artifact — specs first, then the artifact, then verification. The eval suite is the test harness; the workspace iteration is the test-first loop.

## HLD Trace

This LLD traces upstream to:

- **Architecture § Plugins / arrow-maintenance** — names `map-codebase` as the skill that produces skill-creator workspace outputs.
- **Key Design Decisions § The arrow for LID itself** — the behavioral variant (EARS → evals → skill) is the model for how skill-creator operates.

## Component Variant

Behavioral — produces verifiable workspace outputs when invoked. Arrow: HLD → this LLD → EARS → evals + skill implementation.

## Key Components

1. **Workspace layout** — `{skill-name}-workspace/iteration-{N}/` at a location determined by the invoker (typically repo root or `plugins/` parent).
2. **Eval format** — JSON with `id`, `prompt`, `assertions[]` (each with `type` and `value`). Parsed by the runner to produce pass/fail results.
3. **Runner** — calls `claude -p` per eval entry, both with-skill and baseline; computes assertion coverage; writes structured results to workspace.
4. **Promotion gate** — 90% with-skill coverage threshold; documented in iteration summary for auditability.

## Decisions & Alternatives

| Decision | Chosen | Alternatives considered | Rationale |
|---|---|---|---|
| Workspace at caller-specified location | Workspace root or `plugins/` parent | Fixed location under `docs/` | Skills are code artifacts, not documentation; placing workspaces near the plugin they improve is more natural |
| JSON eval format | Flat JSON with `evals` array | YAML; structured Markdown | JSON is unambiguous for machine consumption; YAML requires more parsing; Markdown would complicate assertion extraction |
| 90% promotion threshold | ≥ 90% with-skill assertion coverage | 100%; 80%; no hard threshold | 100% is often impractical (some evals are ambiguous); 80% too loose for quality guarantees; 90% is the community-validated threshold from iteration-1 experience |
| Baseline runs required | Yes — computed alongside with-skill | Optional | Non-discriminating assertions (baseline already passes) are a meaningful quality signal; knowing which assertions are discriminating guides iteration-2 fixture work |

## Open Questions

- **Concurrency default.** The default of 3 parallel eval runs was chosen empirically; it may need adjustment based on Claude rate limits or hardware constraints.
- **Cross-skill eval sharing.** Whether two skills that share reference material should share eval fixtures or maintain independent suites. Current default: independent.
