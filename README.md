# lid-tools

Developer tools for the [LID (Linked-Intent Development)](https://github.com/jszmajda/lid) methodology.

A Rust core engine plus protocol adapters that bring LID into:

- your editor (LSP + VS Code extension),
- your CI (CLI for the `## LID Tooling` hook),
- your AI workflow (MCP server, planned).

## Status

Early development. Building piece by piece toward a `lidc` CLI release, then an LSP, then a VS Code extension.

## Components

| Crate / package        | Purpose                                                  |
| ---------------------- | -------------------------------------------------------- |
| `lid-core`             | Parsing, model, and deterministic coherence checks       |
| `lid-cli` (planned)    | `lidc` binary for CI and the `## LID Tooling` hook       |
| `lid-lsp` (planned)    | Language Server Protocol implementation                  |
| `vscode-lid` (planned) | VS Code extension wrapping `lid-lsp`                     |

## Build

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
