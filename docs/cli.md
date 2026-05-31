---
title: CLI Reference
nav_order: 5
---

# `lidc` CLI Reference

{: .no_toc }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

```
lidc [--root <path>] [--json] <command>
```

---

## Commands

| Command | Description |
|---------|-------------|
| `lidc init [--segment NAME] [--spec-prefix PREFIX]` | Scaffold a new LID project (fails if one already exists) |
| `lidc check` | Run all coherence checks; exit 1 on findings, 2 on error |
| `lidc check --only <ids>` | Run a comma-separated subset of checks |
| `lidc check --fail-on warning` | Promote warnings to failures (default: `error`) |
| `lidc status` | Print segment count and spec coverage (open / implemented / deferred) |

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Clean — no findings at or above the `--fail-on` threshold |
| `1` | Findings at or above the `--fail-on` threshold |
| `2` | Hard error — no repo found, unsupported schema version, or parse error |

---

## Check IDs

Pass one or more IDs (comma-separated) to `--only` to run a subset:

| ID | What it checks |
|----|---------------|
| `schema` | Required fields, valid statuses, known segment references |
| `dag` | No cycles in the `blocks` graph |
| `coverage` | Every `[x]` spec has at least one `@spec` citation in source |
| `spec-id-format` | Spec IDs match the expected pattern; no duplicates |
| `spec-status-counts` | `[x]` specs cited in source, `[D]` specs not cited |
| `orphan` | Every LLD and spec file is reachable from `index.yaml` |
| `reverse-orphan` | Every `@spec` citation refers to a known spec ID |
| `reference-coherence` | Links inside arrow docs resolve to real files |
| `implementing-artifacts` | Implementing artifact paths exist on disk |
| `lld-decisions` | LLD decision tables have at least one authored row |
| `arrow-doc-structure` | Arrow docs contain only recognised section headings |

---

## Quick start

```sh
# 1. Scaffold a new LID project
lidc init                              # creates index.yaml, arrow doc, and intent stubs for "core"
lidc init --segment payments           # custom first segment name
lidc init --segment payments \
          --spec-prefix PAY            # explicit spec-ID prefix (default: uppercased segment name)

# 2. Run coherence checks (use in CI)
lidc check                       # exits 0 = clean, 1 = findings, 2 = error
lidc check --json                # machine-readable output
lidc check --only schema,dag     # run specific checks only

# 3. See spec coverage summary
lidc status
```

---

## CI integration

Add to your CI pipeline (GitHub Actions example):

```yaml
- name: LID coherence check
  run: lidc check
```

`lidc check` exits `0` when clean, `1` when there are findings, and `2` on hard errors — so it integrates cleanly as a quality gate.
