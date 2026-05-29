# Arrow: lid-coach

Advisory principle-review skill for LID projects. Reads a project's LID artifacts, reasons against LID's own principles, and produces a coach-toned report with a posture line, scorecard, and prioritized findings. Auto-invocation disabled; reachable only via `/lid-coach`. Lives inside the `linked-intent-dev` plugin alongside the workflow skill and `update-lid`, but has its own LLD and arrow segment because the embedded principle body and report-shaping requirements are substantial enough to warrant separation.

## Status

**MAPPED** — segment was split out from `linked-intent-dev` on 2026-05-05. Skill body committed; skill-creator iteration-1 run 2026-05-15 (8 fixtures, with-skill 38/38 assertions = 100%, baseline 83%). 18 of 52 LID-COACH specs flipped to `[x]` on iteration-1 eval evidence; 34 remain `[ ]` (not exercised by the current fixture set).

## References

### HLD
- `docs/high-level-design.md` § Architecture / Plugins (linked-intent-dev plugin) — names the coach as one of three skills in the plugin.
- `docs/high-level-design.md` § Key Design Decisions / The arrow for LID itself — establishes the behavioral-skill arrow shape (`HLD → LLD → EARS → evals + SKILL.md + references/`).
- `docs/high-level-design.md` § Approach sections, § Tenets, § Goals, § Key Design Decisions — the canonical source for the coach's embedded principle body. When this content changes, cascade reaches the coach SKILL.md via the LID-on-LID workflow.

### LLD
- `docs/intent/lid-coach/lid-coach-design.md` — this segment's LLD.
- `docs/intent/linked-intent-dev/linked-intent-dev-design.md` — sibling LLD for plugin-level concerns (mode detection, spec ID format, LID-on-LID linkage inversion, eval metadata schema). The coach LLD references this rather than re-specifying.

### EARS
- `docs/intent/lid-coach/lid-coach-specs.md` (52 specs, prefix `LID-COACH-*`)

### Tests
- `plugins/linked-intent-dev/skills/lid-coach/evals/evals.json` — eight prompt fixtures with assertions (unconfigured-project handoff, healthy full-project posture/scorecard/voice, HLD bloat, accumulation antipattern, scoped missing scope, advisory posture, lid-shaped-without-directives, index.yaml-driven arrow sampling).
- skill-creator iteration-1 at `lid-coach-workspace/iteration-1/` (run 2026-05-15): with-skill 38/38 assertions (100%, ±0), baseline 83% (±19). evals 0/3/4/5 are non-discriminating (baseline already passes) — candidates to strengthen in iteration-2.

### Code
- `plugins/linked-intent-dev/skills/lid-coach/SKILL.md` — embedded principle body (description + *why it matters* + audit signal per principle), dispatch table, scorecard format, coach-voice guidance, advisory posture, cold-read pass directive, conversational-mode pointer.
- `plugins/linked-intent-dev/skills/lid-coach/references/lid-faq.md` — load-on-demand conversational guidance covering multi-repo organization, PRDs upstream of HLD, mode-fit changes, the upstream-ownership reframe, and arrow-segment splitting.

The skill is directly invokable as `/lid-coach` — no command stub needed per Claude Code's skills model.

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| Invocation | LID-COACH-001..002 | 0 | 2 | 0 |
| State Dispatch | LID-COACH-003..007 | 2 | 3 | 0 |
| Inputs | LID-COACH-008..013 | 1 | 5 | 0 |
| Principle Body | LID-COACH-014..015 | 0 | 2 | 0 |
| Review Dimensions | LID-COACH-016..029 | 2 | 12 | 0 |
| Mode Interaction | LID-COACH-030..032 | 0 | 3 | 0 |
| Report Structure | LID-COACH-033..038 | 4 | 2 | 0 |
| Advisory Posture | LID-COACH-039..040 | 2 | 0 | 0 |
| Arrow-Maintenance Relationship | LID-COACH-041..042 | 0 | 2 | 0 |
| Voice and Tone | LID-COACH-043..044 | 2 | 0 | 0 |
| Lenient Dispatch | LID-COACH-045..046 | 2 | 0 | 0 |
| Teach While Correcting | LID-COACH-047 | 0 | 1 | 0 |
| Sampling Strategy | LID-COACH-048..049 | 1 | 1 | 0 |
| Quantitative Signals | LID-COACH-050 | 1 | 0 | 0 |
| Cold-Read Pass | LID-COACH-051 | 0 | 1 | 0 |
| Conversational Guidance | LID-COACH-052 | 1 | 0 | 0 |
| **Total** | | **18** | **34** | **0** |
