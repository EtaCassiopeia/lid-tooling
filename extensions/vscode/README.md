# LID — Linked-Intent Development for VS Code

Editor support for the [LID methodology](https://github.com/jszmajda/lid):
hover, go-to-definition, find references, rename, completion, and
real-time diagnostics for `@spec` citations and spec definitions.

## Status

Early development. The extension auto-activates in any workspace
containing `docs/arrows/index.yaml`.

## Features (planned)

- **Hover** — see a spec's text and status by pointing at any
  `@spec ID` reference; see every citation by pointing at a spec
  definition.
- **Go to Definition** — jump from `@spec ID` to the spec line.
- **Find References** — show every site that cites a spec.
- **Rename** — atomically rename a spec ID across the spec file and
  every `@spec` citation.
- **Completion** — autocomplete spec IDs after `@spec ` or `, `.
- **Diagnostics** — real-time error for `@spec` references that don't
  match any defined spec.

## Configuration

| Setting | Description |
| --- | --- |
| `lid.serverPath` | Absolute path to the `lid-lsp` binary. Empty (default) uses the bundled binary. |
| `lid.trace.server` | LSP trace level (`off` / `messages` / `verbose`). |

## License

Dual-licensed under MIT OR Apache-2.0.
