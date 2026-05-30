# LID — Linked-Intent Development

> Keep design intent permanently linked to running code.

LID ([Linked-Intent Development](https://github.com/jszmajda/lid)) is a lightweight methodology that connects every requirement to the test that proves it is met. This extension brings full LID tooling into VS Code — no separate installs required.

## Installation

Search **LID** in the VS Code Extensions panel, or:

```
ext install lid-tools.lid
```

The language server (`lid-lsp`) is bundled inside the extension — nothing else to install.

## Features

### Language Server (LSP)

| Feature | How to trigger |
|---|---|
| **Hover** | Point at any `@spec ID` — see the spec text and status |
| **Go to Definition** | `F12` on a `@spec ID` — jump to the spec line |
| **Find References** | `Shift+F12` on a spec definition — list every citation |
| **Rename** | `F2` on a spec ID — atomically renames across the spec file and all `@spec` citations |
| **Completion** | Type `@spec ` — autocomplete from all known spec IDs |
| **Diagnostics** | Red squiggles on unknown or mismatched `@spec` references; coverage warnings on spec lines |

### Intent Navigator

Open the visual segment graph with **LID: Show Intent Navigator** (`Cmd+Shift+P`).

- **Single-click** a node → read-only detail panel (specs, next/drift, dependencies)
- **Double-click** a node → edit mode: update status, cycle spec status, add specs, edit next/drift prose
- **+ Segment** button → add a new segment via an overlay form
- Live-reloads on any file change (manual edit, MCP agent write, or navigator edit)

### CLI integration

**LID: Initialize Project** scaffolds a new LID project (`docs/arrows/index.yaml`, first segment stub, `docs/intent/`) in the current workspace.

## Commands

| Command | Description |
|---|---|
| `LID: Show Intent Navigator` | Open the visual segment graph |
| `LID: Initialize Project` | Scaffold a new LID project in the workspace |
| `LID: Restart LID Server` | Restart the language server |
| `LID: Show Output Channel` | Open the LID output panel |

## Configuration

| Setting | Default | Description |
|---|---|---|
| `lid.serverPath` | *(bundled)* | Absolute path to `lid-lsp`. Leave empty to use the bundled binary. |
| `lid.trace.server` | `off` | LSP trace verbosity: `off` / `messages` / `verbose` |

## Related tools

| Tool | Install | Purpose |
|---|---|---|
| `lidc` CLI | `brew install EtaCassiopeia/lid/lid` | CI hook, `lidc check`, `lidc init` |
| `lid-mcp` server | `brew install EtaCassiopeia/lid/lid` | MCP server for AI agents (Claude, Cursor) |

See the [repository README](https://github.com/EtaCassiopeia/lid-tooling#readme) for full documentation.

## License

MIT OR Apache-2.0
