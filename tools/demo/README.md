# Demo recording

Two artifacts to produce the GIFs/MP4s for the project README:

| File | What it records | Tool |
| --- | --- | --- |
| `cli.tape` | `lidc check` against the sample project — clean baseline → break → fix | [`vhs`](https://github.com/charmbracelet/vhs) (headless, deterministic) |
| `vscode-demo.applescript` | VS Code: hover, go-to-definition, find-references, rename | macOS Screen Recording + AppleScript |

## CLI demo (terminal only)

```sh
brew install vhs               # or: go install github.com/charmbracelet/vhs@latest
vhs tools/demo/cli.tape
# output: tools/demo/cli.gif
```

Swap the `Output` line at the top of `cli.tape` to write
`cli.mp4` if you'd rather host a video.

The tape runs against `examples/sample-project`, so `lidc` needs
to be on `PATH` (either via `cargo install --path crates/lid-cli`
or aliased to `target/release/lidc`).

## VS Code demo

VHS can't capture VS Code's GUI. The AppleScript drives the
editor with system-level keystrokes; you record the screen
yourself.

### Prep

1. **Build and install** per `WALKTHROUGH.md`:
   ```sh
   cargo build --release
   cd extensions/vscode
   npm ci && npm run compile
   npx vsce package --skip-license
   code --install-extension lid-0.1.0.vsix --force
   ```
2. **Grant accessibility access.** System Settings → Privacy &
   Security → Accessibility → enable the host you're running
   `osascript` from (Terminal, iTerm, or whatever).
3. **Open the sample project** in VS Code first:
   ```sh
   code examples/sample-project
   ```
   The AppleScript assumes the project is already loaded — no
   "open folder" dance.
4. **Reset any prior changes:**
   ```sh
   cd examples/sample-project && git checkout .
   ```

### Record

1. Start the screen recorder: **Cmd+Shift+5** → "Record Selected
   Portion" → drag the rectangle around the VS Code window.
2. Click *Record*.
3. In a separate terminal:
   ```sh
   osascript tools/demo/vscode-demo.applescript
   ```
   Don't touch the keyboard or mouse while it runs — every input
   gets routed to whatever's focused.
4. When the script finishes, click the recording icon in the menu
   bar to stop.

### Tune timings

Each step has an explicit `delay N.N`. Bump them if your machine
is slower (the LSP startup, the hover popup, and the rename apply
are the three sites most likely to flicker by too fast). Trim
once the recording flows.

### Combine clips

A complete README clip is typically the CLI tape (~15 s) followed
by the VS Code clip (~30 s), each looping on the README's `<img>`
tag or embedded in an MP4. To stitch them with `ffmpeg`:

```sh
ffmpeg -i tools/demo/cli.mp4 -i tools/demo/vscode.mov \
    -filter_complex "[0:v][1:v]concat=n=2:v=1:a=0" \
    tools/demo/combined.mp4
```

If you'd rather just produce a GIF for the README's first
viewport, [`gifski`](https://gif.ski/) does a better job than
`ffmpeg`'s built-in GIF encoder on small clips:

```sh
ffmpeg -i tools/demo/vscode.mov -vf fps=15 -f image2pipe -vcodec ppm - |
  gifski --fps 15 --width 1200 -o tools/demo/vscode.gif -
```
