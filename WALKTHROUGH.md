# Local Setup & Testing Walkthrough

End-to-end guide for building the `lid-tools` binaries, installing
the VS Code extension, and exercising every feature against the
bundled sample project.

## 0. Prereqs

- Rust toolchain with `cargo` (verified via `cargo --version`)
- Node 20+ and npm 10+ (verified via `node --version`, `npm --version`)
- VS Code (or another LSP-capable editor — Neovim instructions are
  in section 8)

## 1. Build the binaries

From the repo root (`/Users/gangof3/Projects/lid`):

```sh
cargo build --release
ls -lh target/release/lidc target/release/lid-lsp
```

You should see ~5–15 MB binaries for each. Two ways to use them:

**Option A — install to `~/.cargo/bin` (recommended; puts them on PATH):**

```sh
cargo install --path crates/lid-cli --force
cargo install --path crates/lid-lsp --force
which lidc lid-lsp    # verify
```

**Option B — use them directly without installing:**

```sh
alias lidc=/Users/gangof3/Projects/lid/target/release/lidc
alias lid-lsp=/Users/gangof3/Projects/lid/target/release/lid-lsp
```

## 2. Try the CLI against the example project

```sh
cd /Users/gangof3/Projects/lid/examples/sample-project
lidc check
```

Expected: **`no findings`**. That's the clean baseline.

Try the JSON output:

```sh
lidc check --json | jq .summary
```

Expected:

```json
{
  "findings": 0,
  "by_severity": {},
  "by_check": {}
}
```

Run a single check, or a list:

```sh
lidc check --only schema
lidc check --only coverage,reverse-orphan
```

Promote warnings to failures (useful for CI tightening):

```sh
lidc check --fail-on warning ; echo "exit=$?"
```

## 3. Break something and watch `lidc` catch it

### (a) reverse-orphan — cite a spec that doesn't exist

```sh
cd /Users/gangof3/Projects/lid/examples/sample-project
echo '// @spec AUTH-999' >> src/login.ts
lidc check
# expect: ERRORS (1)  src/login.ts:NN  [reverse-orphan]  @spec AUTH-999 ...
```

Undo:

```sh
sed -i '' -e '$d' src/login.ts
lidc check    # no findings
```

### (b) duplicate spec ID — copy an existing one

```sh
echo '- [x] **AUTH-001**: duplicate' >> docs/specs/auth-specs.md
lidc check
# expect: ERRORS (1)  [spec-id-format]  duplicate spec ID `AUTH-001` ...
sed -i '' -e '$d' docs/specs/auth-specs.md
```

### (c) coverage gap — flip an open spec to `[x]`

```sh
sed -i '' 's/- \[ \] \*\*AUTH-004\*\*/- [x] **AUTH-004**/' docs/specs/auth-specs.md
lidc check
# expect: WARNINGS (1)  [coverage]  spec AUTH-004 is marked [x] but no test cites it
sed -i '' 's/- \[x\] \*\*AUTH-004\*\*/- [ ] **AUTH-004**/' docs/specs/auth-specs.md
```

### (d) cycle in `blocks` — make session block auth

Open `docs/arrows/index.yaml` and change:

```yaml
session:
  blocks: []
```

to:

```yaml
session:
  blocks: [auth]
```

Save, then:

```sh
lidc check
# expect: ERRORS (1)  [dag]  arrow dependency cycle: 2 segments form a cycle (auth → session)
```

Revert the change.

## 4. Build and load the VS Code extension

```sh
cd /Users/gangof3/Projects/lid/extensions/vscode
npm ci                      # if you haven't already
npm run compile
npx vsce package --skip-license
ls -lh lid-0.1.0.vsix
```

Install it into VS Code:

```sh
code --install-extension lid-0.1.0.vsix
```

If `code` isn't on your PATH: open VS Code → **Cmd+Shift+P** →
"Shell Command: Install 'code' command in PATH".

Tell the extension where the server lives. Open your VS Code
settings (Cmd+,) and add:

```jsonc
{
  "lid.serverPath": "/Users/gangof3/Projects/lid/target/release/lid-lsp",
  "lid.trace.server": "verbose"
}
```

Drop the `trace.server` line once everything works — it's noisy.

## 5. Test the extension against the example project

```sh
code /Users/gangof3/Projects/lid/examples/sample-project
```

Watch the status bar (bottom-right):

```
LID: starting…  →  LID ✓
```

If you see `LID: error`, click it to open the output channel and
read the message. Usually means `serverPath` is wrong.

Open **`src/login.ts`** and try each feature:

| Action | Where to click | What to see |
| --- | --- | --- |
| **Hover** | Cursor on `AUTH-001` (inside the `// @spec AUTH-001, AUTH-002` comment) | Popup: `**AUTH-001** — [x] implemented`, the requirement text, "Defined at `docs/specs/auth-specs.md:11`" |
| **Go to Definition** | Cursor on `AUTH-001`, press **F12** | Jumps to `docs/specs/auth-specs.md`, highlights line 11 |
| **Find References** | In the spec file, cursor on `**AUTH-001**`, press **Shift+F12** | Side panel lists: spec definition + `src/login.ts:8` + `tests/login.test.ts:6` |
| **Rename** | Cursor on `AUTH-001`, press **F2**, type `AUTH-LOGIN-001`, press Enter | The spec line, `src/login.ts`, and `tests/login.test.ts` all change in one operation. **Cmd+Z to undo.** |
| **Workspace Symbol** | **Cmd+T**, type `AUTH` | Quick-pick lists `AUTH-001` … `AUTH-005`. Selecting jumps to that line. |

In `src/login.ts`, try the **diagnostics + completion** flow:

| Action | What to type | What to see |
| --- | --- | --- |
| **Diagnostic** | Add a new line at top: `// @spec AUTH-999` | Red squiggle under `AUTH-999`. Hover it → "references a spec ID that is not defined in any spec file". |
| **Completion** | Type `// @spec ` on a new line and pause | Dropdown shows `AUTH-001` … `AUTH-005`. Use arrows + Enter to insert. Each item's preview shows the status badge and requirement text. |
| **Filter** | Continue typing `AUTH-0` | Dropdown narrows to the matching IDs. |

Open `docs/specs/auth-specs.md` to see the **markdown highlighting**:

- The `[x]` / `[ ]` markers tint as constants
- The bold spec IDs tint distinctly
- EARS keywords (`SHALL`, `WHEN`, `WHILE`, `IF`) tint as keywords

Hover on a spec ID in this file → popup shows the citation list
("Cited in 2 places: `src/login.ts:8`, `tests/login.test.ts:6`").

## 6. Use the command palette

**Cmd+Shift+P** → start typing `LID`:

- **LID: Show Output Channel** — focuses the output channel where
  the server logs activity. Useful for debugging.
- **LID: Restart LID Server** — kills and restarts `lid-lsp`.
  Useful after you rebuild the binary (`cargo build --release`)
  to pick up changes without reloading the editor.

## 7. Add a spec end-to-end

To watch the loop close, add `AUTH-006` from scratch:

1. **Add the spec.** Open `docs/specs/auth-specs.md`, add at the bottom:

   ```markdown
   ## Session expiry

   - [ ] **AUTH-006**: When a session token is older than 24 hours, the system SHALL invalidate it on the next request.
   ```

2. **Save.** Open `src/session.ts`. Type `// @spec AUTH-` —
   completion should now offer `AUTH-006`. Pick it.
3. The new line `// @spec AUTH-006` is clean (no squiggle).
4. **Now break it.** Change `AUTH-006` to `AUTH-007` in
   `src/session.ts`. The squiggle reappears (reverse orphan —
   `AUTH-007` doesn't exist).
5. **Fix it.** Cursor on `AUTH-007` → F12 → "no definition found"
   (the LSP knows it's undefined). Change back to `AUTH-006` →
   squiggle disappears.

In a separate terminal:

```sh
cd /Users/gangof3/Projects/lid/examples/sample-project
lidc check
```

At each step, run `lidc check` and watch the count match what the
editor shows.

## 8. Test the LSP without VS Code (Neovim)

If you use Neovim with `nvim-lspconfig`, add to your config:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.lid then
  configs.lid = {
    default_config = {
      cmd = { '/Users/gangof3/Projects/lid/target/release/lid-lsp' },
      filetypes = { 'rust', 'typescript', 'javascript', 'python', 'go', 'java', 'markdown' },
      root_dir = lspconfig.util.root_pattern('docs/arrows/index.yaml'),
      settings = {},
    },
  }
end

lspconfig.lid.setup({})
```

Open the example project in Neovim, navigate to `src/login.ts`,
position the cursor on `AUTH-001`, press `K` for hover, `gd` for
go-to-definition, etc.

## 9. Iterate

When you change the Rust code:

```sh
cargo build --release
# In VS Code: Cmd+Shift+P → "LID: Restart LID Server"
```

The new binary takes effect immediately. When you change the
TypeScript extension code:

```sh
cd extensions/vscode
npm run compile
npx vsce package --skip-license
code --install-extension lid-0.1.0.vsix --force
# Reload the VS Code window (Cmd+Shift+P → "Developer: Reload Window")
```

## 10. Watch the LSP traffic live

In VS Code settings: `"lid.trace.server": "verbose"`. Then
**LID: Show Output Channel** → switch to the "LID Trace" channel.
Every LSP request and response prints. Useful when you're
modifying the LSP handlers and want to see exactly what the editor
is asking for.

---

If anything doesn't behave as expected, capture the relevant
output (terminal + LID output channel) and dig in from there. The
most common gotchas:

- **Status bar stuck at "LID: error"** → `lid.serverPath` is wrong
  or the binary isn't built. Click the indicator → read the
  output channel.
- **No diagnostics on `@spec AUTH-999`** → check that the file's
  language ID matches one of the document selectors. Open the
  Command Palette → "Change Language Mode" to inspect.
- **Hover/rename returns nothing on a spec ID** → the LSP couldn't
  discover the `LidRepo`. Check that `docs/arrows/index.yaml`
  exists at or above the file you're editing.
