# lid-tooling

Developer tools for the [LID (Linked-Intent Development)](https://github.com/jszmajda/lid) methodology — keep design intent permanently linked to running code.

LID answers one recurring problem: *you know what the code does, but you've lost track of why it exists and whether it still does what the design said it should.* This tooling enforces the link between requirements, design docs, and test citations — in your editor, your CI pipeline, and your AI workflow.

---

## Tools

| Tool | What it does |
|------|-------------|
| **VS Code extension** | LSP diagnostics, hover, go-to-definition, rename, completion, visual Intent Navigator |
| **`lidc` CLI** | `lidc check` for CI, `lidc init` to scaffold new projects, `lidc status` for a quick summary |
| **`lid-mcp` MCP server** | 13 tools so AI agents can read and write LID projects with full integrity guarantees |

---

## Installation

### VS Code extension

Search **LID** in the Extensions panel, or run:

```
ext install lid-tools.lid
```

`lid-lsp` is bundled — nothing else to install.

### CLI + MCP server — macOS / Linux (Homebrew)

```sh
brew tap EtaCassiopeia/lid
brew install lid
```

Installs `lidc`, `lid-mcp`, and `lid-lsp` on your `$PATH`.

### CLI + MCP server — macOS / Linux (curl)

```sh
# lidc only
curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash

# lidc + lid-mcp
curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash -s -- --mcp
```

### CLI + MCP server — Windows

```powershell
# lidc only
irm https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.ps1 | iex

# lidc + lid-mcp
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.ps1))) -Mcp
```

### From source

```sh
cargo install --git https://github.com/EtaCassiopeia/lid-tooling lidc lid-mcp
```

---

## Quick start

```sh
# 1. Scaffold a new LID project
lidc init                        # creates docs/arrows/index.yaml + first segment stub
lidc init --segment payments     # custom first segment name

# 2. Run coherence checks (use in CI)
lidc check                       # exits 0 = clean, 1 = findings, 2 = error
lidc check --json                # machine-readable output
lidc check --only schema,dag     # run specific checks only

# 3. See spec coverage summary
lidc status
```

---

## `lidc` reference

```
lidc [--root <path>] [--json] <command>
```

| Command | Description |
|---------|-------------|
| `lidc init [--segment NAME]` | Scaffold a new LID project (fails if one already exists) |
| `lidc check` | Run all coherence checks; exit 1 on findings, 2 on error |
| `lidc check --only <ids>` | Run a comma-separated subset of checks |
| `lidc check --fail-on warning` | Promote warnings to failures (default: `error`) |
| `lidc status` | Print segment count and spec coverage (open / implemented / deferred) |

**Check IDs:** `schema`, `dag`, `coverage`, `spec-id-format`, `spec-status-counts`,
`orphan`, `reverse-orphan`, `reference-coherence`, `implementing-artifacts`,
`lld-decisions`, `arrow-doc-structure`

**Exit codes:** `0` clean · `1` findings at or above `--fail-on` threshold · `2` hard error (no repo, unsupported schema version, parse error)

---

## VS Code extension

The extension auto-activates in any workspace containing `docs/arrows/index.yaml`.

### Language server features

| Feature | Trigger |
|---------|---------|
| **Hover** | Point at `@spec ID` — shows spec text and status |
| **Go to Definition** | `F12` on `@spec ID` — jumps to the spec line |
| **Find References** | `Shift+F12` on a spec definition — lists every citation |
| **Rename** | `F2` on a spec ID — renames across the spec file and all `@spec` citations |
| **Completion** | Type `@spec ` — autocompletes from all known IDs |
| **Diagnostics** | Squiggles on unknown `@spec` references; coverage warnings on spec lines |

### Intent Navigator

Open with **LID: Show Intent Navigator** (`Cmd+Shift+P`).

- **Single-click** a segment node → read-only detail panel (status, specs, dependencies, next/drift prose)
- **Double-click** a segment node → edit mode: update status, cycle spec status, edit next/drift
- **`+ Segment`** button → add a new segment via overlay form
- **`+ Add spec`** → append a spec line directly from the panel
- Live-reloads on any file change — whether from the navigator, a manual edit, or an MCP agent write

### Commands

| Command | Description |
|---------|-------------|
| `LID: Show Intent Navigator` | Open the visual segment graph |
| `LID: Initialize Project` | Scaffold a new LID project in the workspace |
| `LID: Restart LID Server` | Restart the language server |
| `LID: Show Output Channel` | Open the LID output panel |

### Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `lid.serverPath` | *(bundled)* | Override path to `lid-lsp` binary |
| `lid.trace.server` | `off` | LSP trace verbosity: `off` / `messages` / `verbose` |

---

## MCP server

`lid-mcp` is a stdio MCP server — AI agents call its tools to inspect and modify a LID project with the same integrity guarantees as the LSP.

### Configuration

**`~/Library/Application Support/Claude/claude_desktop_config.json`** (macOS):
```json
{
  "mcpServers": {
    "lid": { "command": "lid-mcp" }
  }
}
```

**`.mcp.json`** at project root (Cursor, VS Code, other MCP clients):
```json
{
  "servers": {
    "lid": { "type": "stdio", "command": "lid-mcp" }
  }
}
```

### Tools

Always call `lid_discover` first to register the project, then use any other tool.

| Tool | Description |
|------|-------------|
| `lid_init` | Scaffold a new LID project at a path |
| `lid_discover` | Find and register a LID project; returns root and segment count |
| `lid_status` | Segment overview with spec counts |
| `lid_check` | Run coherence checks; returns all findings |
| `lid_list_segments` | List all segments with status and detail path |
| `lid_get_segment` | Full detail for one segment including spec lines |
| `lid_list_specs` | All spec lines; optional segment-ID prefix filter |
| `lid_find_spec_references` | Source-code locations citing `@spec SPEC-ID` |
| `lid_search` | Case-insensitive search across segment IDs, spec IDs, and text |
| `lid_add_segment` | Add a new segment to the arrow index |
| `lid_update_segment` | Update status, next, or drift on an existing segment |
| `lid_add_spec` | Append a new open spec line to a segment |
| `lid_update_spec_status` | Mark a spec open / implemented / deferred |

---

## Project layout (schema v2)

```
docs/
├── arrows/
│   ├── index.yaml            # segment graph — statuses, blocks, taxonomy
│   └── <segment>/
│       └── *.md              # arrow detail docs (HLDs, LLDs)
└── intent/
    └── <segment>/
        ├── <segment>-specs.md    # spec lines  (- [ ] **ID**: text)
        └── <segment>-design.md   # design docs
```

`index.yaml` minimal shape:

```yaml
schema_version: 2
arrows:
  auth:
    status: MAPPED          # UNMAPPED | MAPPED | AUDITED | OK | MERGED
    detail: auth/core.md
```

Spec line format:

```markdown
- [ ] **AUTH-001**: the system shall authenticate users via password.
- [x] **AUTH-002**: sessions shall expire after 30 minutes of inactivity.
- [D] **AUTH-003**: biometric login is deferred to v2.
```

Source citation format (any language):

```rust
// @spec AUTH-002
fn validate_session_expiry(...) { ... }
```

---

## Building from source

```sh
# Rust workspace
cargo build --release
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# VS Code extension
cd extensions/vscode
npm ci && npm run compile
```

The pre-push hook runs `cargo fmt --check` and `cargo test` automatically.

---

## Versioning

lid-tooling versions independently of the upstream LID project.
Compatibility with `schema_version` in `docs/arrows/index.yaml`:

| lid-tooling | `schema_version` |
|-------------|-----------------|
| 0.2.x | 2 |

Encountering an unsupported schema version produces a hard error with a migration hint — never a silent partial load.

---

## License

MIT OR Apache-2.0 — see [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
