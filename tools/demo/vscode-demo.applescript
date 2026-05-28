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
--   * Open `examples/sample-project` in VS Code beforehand --
--     the script assumes that workspace is already loaded.
--   * Grant Accessibility access: System Settings -> Privacy &
--     Security -> Accessibility -> enable the host that's running
--     `osascript` (Terminal or iTerm).
--   * Start your screen recorder (Cmd+Shift+5) before running.
--
-- Pacing notes: every action is followed by an explicit `delay`.
-- The defaults below are tuned for a viewer who's never seen the
-- feature before to actually read what's on screen -- bump them
-- another 25--50% if you're recording at a higher frame rate or
-- on a slow machine. Trim once your first take looks fluent.

on quickOpen(filename)
    tell application "System Events"
        keystroke "p" using {command down}
        delay 0.8
        keystroke filename
        delay 1.0
        key code 36 -- Return
    end tell
    delay 1.8
end quickOpen

on goToLine(spec)
    -- VS Code on macOS: "Go to Line/Column..." is Ctrl+G
    -- (control only, not command -- Cmd+G is Find Next).
    tell application "System Events"
        keystroke "g" using {control down}
        delay 0.6
        keystroke spec
        delay 0.6
        key code 36 -- Return
    end tell
    delay 1.2
end goToLine

on pressEscape()
    tell application "System Events" to key code 53
    delay 0.7
end pressEscape

-- == Activate VS Code ===========================================

tell application "Visual Studio Code" to activate
delay 2.5 -- let the viewer recognise the window before anything happens

-- == 1. Hover on @spec AUTH-001 in source =======================

quickOpen("src/login.ts")

-- Cursor at line 8, column 12 -> inside `AUTH-001` in
-- `// @spec AUTH-001, AUTH-002`
goToLine("8:12")

-- Cmd+K, Cmd+I -> Show Hover
tell application "System Events"
    keystroke "k" using {command down}
    delay 0.2
    keystroke "i" using {command down}
end tell
delay 5 -- pause for the camera to capture the popup (spec text,
         -- status badge, "Defined at ..." link)

pressEscape()

-- == 2. Go to Definition ========================================

tell application "System Events" to key code 111 -- F12
delay 4 -- target file opens; viewer reads the highlighted line

-- == 3. Find All References on the spec definition ==============

tell application "System Events" to key code 111 using {shift down}
delay 5 -- side panel opens with the citation list

pressEscape()

-- == 4. Rename across the workspace =============================

-- Re-anchor the cursor on the bold spec ID (Shift+F12 may have
-- shifted focus into the panel).
quickOpen("docs/specs/auth-specs.md")
goToLine("11:11") -- inside `**AUTH-001**`

tell application "System Events"
    key code 120 -- F2 (rename)
    delay 1.5 -- rename input pops up
    keystroke "AUTH-LOGIN-001"
    delay 1.5 -- viewer reads the new name in the input
    key code 36 -- Return -> apply rename
end tell
delay 5 -- pause so the spec line + the two citation files all
        -- visibly update before the next action

-- == 5. Hop into the source file to show the citation updated ==

quickOpen("src/login.ts")
delay 4 -- final hold so the rename's effect is the closing frame

-- End of demo. Stop the recorder manually.
-- To reset the project for another take:
--   cd examples/sample-project && git checkout .
