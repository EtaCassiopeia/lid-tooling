# lid-tools

Developer tools for the [LID (Linked-Intent Development)](https://github.com/jszmajda/lid) methodology.

A Rust core engine plus protocol adapters that bring LID into:

- your editor (LSP + VS Code extension),
- your CI (CLI for the `## LID Tooling` hook),
- your AI workflow (MCP server, planned).

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

## Components

| Crate / package | Purpose | Status |
| --- | --- | --- |
| `lid-core` | Parsing, model, and 9 deterministic coherence checks | ✅ shipping |
| `lid-cli` | `lidc` binary — CI hook and the `## LID Tooling` slot | ✅ shipping |
| `lid-lsp` | Language Server: hover, definition, references, rename, completion, diagnostics | ✅ shipping |
| `vscode-lid` | VS Code extension bundling the language client | ✅ shipping |

## Quick start

```sh
# 1. Build
cargo build --release
cargo install --path crates/lid-cli --force
cargo install --path crates/lid-lsp --force

# 2. Try the CLI against the bundled sample project
cd examples/sample-project
lidc check

# 3. Build and install the VS Code extension
cd ../../extensions/vscode
npm ci && npm run compile
npx vsce package --skip-license
code --install-extension lid-0.1.0.vsix
```

Full walkthrough with test scenarios (hover, rename, diagnostics,
deliberate breakage): [`WALKTHROUGH.md`](WALKTHROUGH.md).

## Build & test

```sh
cargo build
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

CI runs the same matrix plus a VS Code-extension build (`npm run
compile` + `vsce package` smoke test).

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
