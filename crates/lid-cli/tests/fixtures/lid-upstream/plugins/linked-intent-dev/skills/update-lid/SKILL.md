# Skill: update-lid

Bootstrap and idempotent reconciliation for LID project configuration.

<!-- @spec UPDATE-LID-001 -->
When the user invokes `/update-lid`, dispatch to this skill. This skill is also reachable as a sub-step from `/linked-intent-dev` (when the workflow detects an unconfigured project) and from `/map-codebase` at its terminal verification step.

## State Dispatch

<!-- @spec UPDATE-LID-002 -->
If the project has no `CLAUDE.md` and no `docs/` directory, perform a full bootstrap: create required directories, then create `CLAUDE.md` with LID directives and a mode marker.

<!-- @spec UPDATE-LID-003 -->
If the project has an existing `CLAUDE.md` with no LID directives, append LID directives without overwriting or removing existing content.

<!-- @spec UPDATE-LID-004 -->
If the project has LID directives but no `## LID Mode:` heading, add the heading with the default mode (Full).

<!-- @spec UPDATE-LID-005 -->
If the project is fully configured with no mode change requested, check for convention drift (missing required directories, missing required files, outdated directive sections) and surface each detected difference as a proposed update requiring user confirmation.

<!-- @spec UPDATE-LID-006 -->
If invoked with an explicit mode change request, execute the appropriate mode transition (promotion or demotion).

## Mode Interaction

<!-- @spec UPDATE-LID-007 -->
During a full bootstrap, prompt the user for the intended mode before writing the mode marker, unless the caller has already determined the mode — in that case use the caller-provided mode without re-prompting.

<!-- @spec UPDATE-LID-008 -->
When the user does not explicitly specify a mode during bootstrap, select Full LID.

<!-- @spec UPDATE-LID-009 -->
Persist the selected mode under a heading of the form `## LID Mode: {Full|Scoped}` in the project's `CLAUDE.md`.
