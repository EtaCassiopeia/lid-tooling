-- AppleScript demo that drives Visual Studio Code through the
-- LID extension's headline features: hover, go-to-definition,
-- find references, and workspace-wide rename.
--
-- Run from Script Editor or via osascript:
--
--   osascript tools/demo/vscode-demo.applescript
--
-- Prereqs:
--   * Build the binaries (`cargo build --release`) and install
--     the extension per WALKTHROUGH.md.
--   * Open `examples/sample-project` in VS Code beforehand —
--     the script assumes that workspace is already loaded.
--   * Grant Accessibility access: System Settings → Privacy &
--     Security → Accessibility → enable the host that's running
--     `osascript` (Terminal or iTerm).
--   * Start your screen recorder (Cmd+Shift+5) before running.
--
-- Timing notes: every action is followed by an explicit `delay`.
-- Bump the values if your machine is slower; trim them once the
-- recording looks fluent.

on quickOpen(filename)
    tell application "System Events"
        keystroke "p" using {command down}
        delay 0.6
        keystroke filename
        delay 0.8
        key code 36 -- Return
    end tell
    delay 1.5
end quickOpen

on goToLine(spec)
    tell application "System Events"
        keystroke "g" using {command down, control down}
        delay 0.5
        keystroke spec
        delay 0.4
        key code 36 -- Return
    end tell
    delay 1.0
end goToLine

on pressEscape()
    tell application "System Events" to key code 53
    delay 0.5
end pressEscape

-- ── Activate VS Code ───────────────────────────────────────────

tell application "Visual Studio Code" to activate
delay 1.5

-- ── 1. Hover on @spec AUTH-001 in source ───────────────────────

quickOpen("src/login.ts")

-- Cursor at line 8, column 12 → inside `AUTH-001` in
-- `// @spec AUTH-001, AUTH-002`
goToLine("8:12")

-- Cmd+K, Cmd+I → Show Hover
tell application "System Events"
    keystroke "k" using {command down}
    delay 0.15
    keystroke "i" using {command down}
end tell
delay 3.5 -- pause for the camera to capture the popup

pressEscape()

-- ── 2. Go to Definition ────────────────────────────────────────

tell application "System Events" to key code 111 -- F12
delay 3.0

-- ── 3. Find All References on the spec definition ──────────────

tell application "System Events" to key code 111 using {shift down}
delay 4.0

pressEscape()

-- ── 4. Rename across the workspace ─────────────────────────────

-- Re-anchor the cursor on the bold spec ID (Shift+F12 may have
-- shifted focus into the panel).
quickOpen("docs/specs/auth-specs.md")
goToLine("11:11") -- inside `**AUTH-001**`

tell application "System Events"
    key code 120 -- F2 (rename)
    delay 1.2
    keystroke "AUTH-LOGIN-001"
    delay 1.0
    key code 36 -- Return → apply rename
end tell
delay 3.5 -- pause to show every site updating

-- ── 5. Hop into the source file to show the citation updated ──

quickOpen("src/login.ts")
delay 2.5

-- End of demo. Stop the recorder manually.
-- To reset the project for another take:
--   cd examples/sample-project && git checkout .
