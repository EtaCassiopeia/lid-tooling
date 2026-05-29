# Arrow: bidirectional-differential

The first `lid-experimental` experiment — audits EARS↔code coherence by spawning two parallel fresh `claude -p` sessions per spec (A-direction reconstructs code from EARS; B-direction reconstructs EARS from stripped code) and classifies the drift between their outputs and the real artifacts.

## Status

**AUDITED** — bootstrapped on 2026-04-25 (git SHA `b64c439`). Audited 2026-05-18. All 22 BIDIFF specs implemented and marked `[x]`. One residual drift: BIDIFF-007's prose references a reference file (`stripping-rules.md`) that was consolidated into `audit-protocol.md`.

## References

### HLD
- `docs/high-level-design.md` § Key Design Decisions / Experimental features as a separate plugin (governs all experiments under `lid-experimental`)

### LLD
- `docs/intent/bidirectional-differential/bidirectional-differential-design.md` (sub-LLD)
- `docs/intent/lid-experimental/lid-experimental-design.md` § Active Experiments / bidirectional-differential (parent container's pointer)

### EARS
- `docs/intent/bidirectional-differential/bidirectional-differential-specs.md` (22 specs, prefix `BIDIFF-*`)

### Tests
- `plugins/lid-experimental/skills/bidirectional-differential/evals/evals.json` (3 baseline fixtures: `bd-coherent-bounded-matrix`, `bidirectional-drift-missing-subdecision`, `b-only-drift-unstated-invariant`)
- `plugins/lid-experimental/skills/bidirectional-differential-workspace/iteration-1/` — workspace runs against the three eval fixtures

### Code
- `plugins/lid-experimental/skills/bidirectional-differential/SKILL.md`
- `plugins/lid-experimental/skills/bidirectional-differential/references/` — `audit-protocol.md` (full six-step protocol; includes stripping rules and split-result mechanics referenced from individual specs), `classification-codes.md`, `scoping-conversation.md`, `audit-report-template.md`
- `plugins/lid-experimental/commands/differential-audit.md` (plugin-level command stub)

## Architecture

**Purpose:** Detect EARS↔code drift that formal/structural engines (type systems, CodeQL, test-witness, LemmaScript) cannot see — operational-halo drift, decomposition gaps, missing sub-decisions, unstated invariants. Two-direction blind reconstruction: A-direction generates code from EARS only; B-direction reconstructs EARS from stripped code only. Within-direction variance plus between-direction alignment yields one of six classification codes (`BD-COHERENT`, `A-ONLY-DRIFT`, `B-ONLY-DRIFT`, `BIDIRECTIONAL-DRIFT`, `INCONSISTENT-BLIND`, `UNANNOTATABLE`).

**Key Components:**
1. Scoping conversation — natural-language → arrow → EARS mapping; cost-estimate confirmation before spawning subprocesses.
2. Six-step audit protocol — input resolution, identifier stripping, parallel `claude -p` spawns (N runs per direction, default 3), classification, per-EARS audit-record write.
3. Reserved output subtree — `docs/arrows/_experiments/bidirectional-differential/` (sanctioned namespace per `arrow-maintenance` LLD § *Experiment-produced artifacts*).

## Spec Coverage

| Category | Spec range | Implemented | Active gap | Deferred |
|---|---|---|---|---|
| All BIDIFF | BIDIFF-001..023 (no -004) | 22 | 0 | 0 |

**Summary:** 22 of 22 active specs implemented.
