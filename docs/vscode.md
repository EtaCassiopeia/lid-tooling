---
title: VS Code Extension
nav_order: 3
---

# VS Code Extension

{: .no_toc }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

The extension auto-activates in any workspace containing `docs/arrows/index.yaml`.

Install from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=lid-tools.lid) — see [Installation]({{ site.baseurl }}/installation#vs-code-extension) for all methods.

---

## Language server features

| Feature | Trigger |
|---------|---------|
| **Hover** | Point at `@spec ID` — shows spec text and status |
| **Go to Definition** | `F12` on `@spec ID` — jumps to the spec line |
| **Find References** | `Shift+F12` on a spec definition — lists every citation |
| **Rename** | `F2` on a spec ID — renames across the spec file and all `@spec` citations |
| **Completion** | Type `@spec ` — autocompletes from all known IDs |
| **Diagnostics** | Squiggles on unknown `@spec` references; coverage warnings on spec lines |

---

## Intent Navigator

Open with **LID: Show Intent Navigator** (`Cmd+Shift+P`).

- **Single-click** a segment node → read-only detail panel (status, specs, dependencies, next/drift prose)
- **Double-click** a segment node → edit mode: update status, cycle spec status, edit next/drift
- **`+ Segment`** button → add a new segment via overlay form; scaffolds `index.yaml` entry, arrow doc stub, and intent files (`*-specs.md` with `prefix:` frontmatter, `*-design.md`); the Spec Prefix field is pre-filled by majority-namespace inference and updates live as you type the segment ID
- **`+ Add spec`** → append a spec line directly from the panel
- Live-reloads on any file change — whether from the navigator, a manual edit, or an MCP agent write

---

## Commands

| Command | Description |
|---------|-------------|
| `LID: Show Intent Navigator` | Open the visual segment graph |
| `LID: Initialize Project` | Scaffold a new LID project in the workspace |
| `LID: Restart LID Server` | Restart the language server |
| `LID: Show Output Channel` | Open the LID output panel |

---

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `lid.serverPath` | *(bundled)* | Override path to `lid-lsp` binary |
| `lid.trace.server` | `off` | LSP trace verbosity: `off` / `messages` / `verbose` |
