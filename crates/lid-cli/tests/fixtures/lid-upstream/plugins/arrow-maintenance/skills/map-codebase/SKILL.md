# Skill: map-codebase

Map an existing codebase to a LID arrow structure. Produces `docs/arrows/index.yaml`, arrow docs, intent directories, and spec stubs from analysis of the codebase.

<!-- @spec SKILL-CREATOR-001 -->
When the user invokes `/map-codebase`, dispatch to this skill.

<!-- @spec SKILL-CREATOR-002 -->
Accept as inputs: a target skill name, a path to an EARS spec file, and an optional reference SKILL.md.

<!-- @spec SKILL-CREATOR-003 -->
When no reference SKILL.md is provided, generate an initial SKILL.md from the EARS spec file alone.

## Workspace Layout

<!-- @spec SKILL-CREATOR-004 -->
Create a workspace directory at `{skill-name}-workspace/iteration-{N}/` where N is the next iteration number.

<!-- @spec SKILL-CREATOR-005 -->
Write the candidate SKILL.md, eval results summary, and a diff from the previous iteration into the workspace directory.

## Codebase Analysis

When invoked on an existing project (not a skill-creator workspace invocation), analyze the project structure to:

1. Identify logical segments from directory structure and naming conventions
2. Propose arrow entries for each identified segment
3. Generate stub spec files for each arrow entry
4. Propose `blocks`/`blockedBy` relationships from import graphs and naming

The output is a draft `docs/arrows/index.yaml` and a set of stub arrow docs and intent directories. All proposed content requires user review and confirmation before being written.
