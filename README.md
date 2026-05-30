# lid-tools

Developer tools for the [LID (Linked-Intent Development)](https://github.com/jszmajda/lid) methodology.

A Rust core engine plus protocol adapters that bring LID into your editor, your CI pipeline, and your AI workflow.

## Demo

### `lidc check` from the terminal

![lidc CLI demo](tools/demo/cli.gif)

Above: `lidc check` running against the bundled
[sample project](examples/sample-project/) — clean baseline →
planted reverse-orphan → JSON summary → filtered check → fix.

### VS Code extension

The companion VS Code extension wraps `lid-lsp` and gives editor
users hover, go-to-definition, find-references, autocomplete,
diagnostics, and workspace-wide rename for `@spec` citations:

📹 **[Watch the VS Code demo (tools/demo/vscode.mp4)](tools/demo/vscode.mp4)** — ~85 s, six scenes
walking through every feature against the sample project.

See [`WALKTHROUGH.md`](WALKTHROUGH.md) for the full end-to-end
setup (CLI + LSP + VS Code extension) and
[`tools/demo/README.md`](tools/demo/README.md) for how the demos
are recorded.

## Installation

### VS Code extension (recommended — includes everything)

Search **LID** in the Extensions panel, or:

```
ext install lid-tools.lid
```

`lid-lsp` is bundled inside the extension — nothing else to install.

### CLI + MCP server — macOS / Linux

```sh
brew tap EtaCassiopeia/lid
brew install lid
```

This puts `lidc`, `lid-mcp`, and `lid-lsp` on your `$PATH`.

### CLI + MCP server — Windows

```powershell
irm https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.ps1 | iex
```

Add `--Mcp` to also install `lid-mcp`.

### One-line install script (Linux / macOS without Homebrew)

```sh
# lidc only
curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash

# lidc + lid-mcp
curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash -s -- --mcp
```

### From source

```sh
cargo install --git https://github.com/EtaCassiopeia/lid-tooling lidc lid-mcp
```

---

## MCP server configuration

After installing `lid-mcp`, add it to your MCP client:

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "lid": { "command": "lid-mcp" }
  }
}
```

**Cursor / VS Code** (`.mcp.json` at project root):
```json
{
  "servers": {
    "lid": { "type": "stdio", "command": "lid-mcp" }
  }
}
```

---

## Components

| Crate / package | Purpose |
| --- | --- |
| `lid-core` | Parsing, model, and coherence checks |
| `lid-cli` | `lidc` binary — `lidc check`, `lidc init`, CI hook |
| `lid-lsp` | Language server: hover, definition, references, rename, completion, diagnostics |
| `lid-mcp` | MCP server: 13 tools for AI agents to read and write LID projects |
| `extensions/vscode` | VS Code extension bundling `lid-lsp` |

---

## Build & test

```sh
cargo build
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

CI runs the same matrix plus a VS Code-extension build (`npm run
compile` + `vsce package` smoke test).

## Versioning & Compatibility

lid-tooling follows its own [semantic versioning](https://semver.org/) independent
of the upstream LID project. Compatibility is expressed through `SUPPORTED_SCHEMA_VERSIONS`
in `lid-core`: the tool hard-errors with a migration hint when it encounters a
`schema_version` it cannot handle, so you never get a silently broken repo load.

`schema_version` in `docs/arrows/index.yaml` is the machine-readable compatibility
signal — not the lid-tooling package version.

| lid-tooling | Supported `schema_version` |
|-------------|---------------------------|
| 0.2.x       | 2                         |
| 0.1.x       | —  (pre-release)          |

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
