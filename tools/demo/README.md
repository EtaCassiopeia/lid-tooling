# Demo recording

Scripted, repeatable demo recordings for the project README. Two
artifacts, two recorders, two outputs:

| Demo | Source | Recorder | Output |
| --- | --- | --- | --- |
| CLI | `cli.tape` | `vhs` (headless) | `cli.gif` |
| VS Code | `vscode-demo.applescript` + `record-vscode.sh` | `ffmpeg` + osascript | `vscode.mp4` |

Both run against `examples/sample-project/`. After either recording
finishes, the sample project is reset to clean.

---

## CLI demo (`cli.tape`)

A headless terminal recording. Reproducible bit-for-bit; no GUI,
no screen capture, no permissions.

### Install once

```sh
brew install vhs
# or: go install github.com/charmbracelet/vhs@latest
```

VHS also needs Google Chrome installed (it drives a headless
Chrome to render the terminal); on Macs that ship with Chrome
this just works.

### Build the binary VHS will record

```sh
cargo install --path crates/lid-cli --force
which lidc   # confirm it's on PATH
```

### Record

From the repo root:

```sh
vhs tools/demo/cli.tape
```

Output: `tools/demo/cli.gif` (currently ~145 KB, 22 s, 1200×720).

### Tweak

Open `tools/demo/cli.tape`. Knobs:

- `Set TypingSpeed 75ms` — how fast commands appear.
- `Set Theme "Catppuccin Mocha"` — any [VHS theme name](https://github.com/charmbracelet/vhs/tree/main/themes).
- `Set Width / Height` — pixels.
- `Sleep N` between commands — viewer reading time.
- Output as `cli.mp4` instead of `cli.gif` by changing the
  `Output` line at the top.

---

## VS Code demo (`record-vscode.sh`)

A real screen recording driven by:

- `vscode-demo.applescript` — sends keystrokes to VS Code via
  `osascript` / System Events
- `record-vscode.sh` — orchestrates: reset project, open
  workspace, activate VS Code, start ffmpeg, fire the AppleScript

### Install once

```sh
brew install ffmpeg
```

Plus the `code` CLI:

> In VS Code: Cmd+Shift+P → "Shell Command: Install 'code' command in PATH"

### Grant permissions once

System Settings → Privacy & Security:

1. **Accessibility** → toggle ON for the terminal you'll run the
   script from (Warp, iTerm, or Terminal). This lets `osascript`
   send keystrokes to other apps.
2. **Screen & System Audio Recording** → same terminal, toggle ON.
   This lets `ffmpeg` capture the screen.

The first time `osascript` and `ffmpeg` run, macOS may pop up
dialogs — click *Allow*. After that, they stay granted.

### Set up VS Code once

1. Build the binaries and install the extension per [`WALKTHROUGH.md`](../../WALKTHROUGH.md).
2. Configure `lid.serverPath` to your `lid-lsp` binary.
3. Open the sample project: `code examples/sample-project`.
4. **Reload the window** (Cmd+Shift+P → "Developer: Reload Window")
   so the workspace's `.vscode/settings.json` takes effect. That
   file sets `screencastMode.onlyKeyboardShortcuts: true` so the
   recorded overlay shows shortcut keys (`F12`, `F2`, etc.) and
   not every individual keystroke.

To confirm the screencast setting loaded, press `Cmd+P` and close
it — the overlay should briefly show `Cmd+P`. Type some plain
text — it should *not* appear in the overlay.

### Record

From the repo root, with VS Code already open on the sample
project:

```sh
tools/demo/record-vscode.sh
```

The script:

1. Resets `examples/sample-project/` to a clean baseline (`git checkout .`).
2. Opens the workspace (idempotent — focuses the existing window).
3. Activates VS Code via `osascript`.
4. Counts down 6 seconds. **If VS Code didn't come to the front, switch to it now.**
5. Starts `ffmpeg` capturing screen 0 for 95 seconds.
6. Fires the AppleScript demo (~85 s of action). **Don't touch the keyboard or mouse during this window.**
7. Waits for ffmpeg to finish.
8. Resets the sample project again (the rename + the deliberately bad citation leave it dirty).

Output: `tools/demo/vscode.mp4` (~4.5 MB, ~95 s, full screen).

### What the demo shows

Six scenes, all with a `goToLine` re-anchoring the cursor on the
relevant spec ID immediately before the shortcut, so the LSP
always has a spec ID at the cursor:

| Scene | Shortcut | What it demonstrates |
| --- | --- | --- |
| 1. Hover | `Cmd+K Cmd+I` | Spec text, status, and "Defined at" link on `@spec AUTH-001` |
| 2. Go to Definition | `F12` | Jump from a citation to the spec line |
| 3. Find References | `Shift+F12` | Side panel lists every site that cites the spec |
| 4. Autocomplete | typing `// @spec ` | Completion popup with project specs and status badges |
| 5. Diagnostic | typing `AUTH-999` | Red squiggle, then `Cmd+K Cmd+I` to surface the error |
| 6. Rename | `F2` | `AUTH-001` → `AUTH-LOGIN-001` across spec + every citation atomically |

### Tweak

Open `tools/demo/vscode-demo.applescript`. Each scene has its own
section. The `delay N` after each action is the reading time;
adjust per scene. The helpers:

- `quickOpen(filename)` — `Cmd+P`, type, Enter.
- `goToLine(spec)` — `Ctrl+G`, type `LINE:COL`, Enter.
- `slowType(text, perCharDelay)` — types character-by-character so
  the autocomplete popup is captured by the recorder.

For environment knobs (recording duration, warmup countdown), edit
`record-vscode.sh` or override:

```sh
DEMO_SECONDS=120 DEMO_WARMUP=10 tools/demo/record-vscode.sh
```

### Convert MP4 → GIF (optional)

If you want the VS Code demo inline in the README, convert to GIF:

```sh
# Two-pass with palette generation (smaller, better quality)
ffmpeg -y -i tools/demo/vscode.mp4 \
    -vf "fps=12,scale=1200:-1:flags=lanczos,palettegen=stats_mode=diff" \
    /tmp/palette.png

ffmpeg -y -i tools/demo/vscode.mp4 -i /tmp/palette.png \
    -filter_complex "fps=12,scale=1200:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
    tools/demo/vscode.gif
```

Or via [`gifski`](https://gif.ski/) for even better quality (`brew install gifski`):

```sh
ffmpeg -i tools/demo/vscode.mp4 -vf "fps=15,scale=1200:-1" -f image2pipe -vcodec ppm - |
    gifski --fps 15 --width 1200 -o tools/demo/vscode.gif -
```

---

## Troubleshooting

**ffmpeg fails with "Input/output error"**
- Screen Recording permission is not granted for the terminal.
- Or another ffmpeg/`screencapture` is still recording — `pgrep -fl ffmpeg`.

**AppleScript fails with "System Events got an error: ... not allowed to send keystrokes"**
- Accessibility permission is not granted for the terminal.

**Screencast overlay still shows every keystroke**
- The workspace `.vscode/settings.json` didn't load. Cmd+Shift+P →
  "Developer: Reload Window", then verify by pressing `Cmd+P` —
  you should see the binding, not the literal `p`.

**Cursor lands somewhere unexpected and a shortcut returns "no symbol at position"**
- The `goToLine` call before that shortcut is targeting the wrong
  line. Adjust the `LINE:COL` argument in the AppleScript. The
  current values match the on-disk `examples/sample-project/`
  layout; if you edit the sample files, fix the line numbers.

**Notifications captioning was removed**
- An earlier version of the script fired `display notification`
  banners before each scene. They added little, especially with
  the Screencast Mode overlay already labeling each shortcut, so
  the current script omits them.
