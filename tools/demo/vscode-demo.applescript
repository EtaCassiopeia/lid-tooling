-- AppleScript demo that drives Visual Studio Code through the
-- LID extension's headline features: hover, go-to-definition,
-- find references, autocomplete, diagnostics, and workspace-wide
-- rename.
--
-- Run via the shell wrapper:
--
--   tools/demo/record-vscode.sh
--
-- Prereqs:
--   * Build the binaries (`cargo build --release`) and install
--     the extension per WALKTHROUGH.md.
--   * The sample project's `.vscode/settings.json` sets Screencast
--     Mode to keys-only so the overlay reads cleanly.
--   * Grant Accessibility to whatever shell runs osascript.
--
-- Runtime ~80 s. Every spec-ID-aware shortcut is preceded by an
-- explicit `goToLine` so the LSP always sees the cursor on the
-- spec ID before processing the request.

on quickOpen(filename)
    tell application "System Events"
        keystroke "p" using {command down}
        delay 0.9
        keystroke filename
        delay 1.0
        key code 36 -- Return
    end tell
    delay 2.2
end quickOpen

on goToLine(spec)
    -- VS Code on macOS: "Go to Line/Column..." is Ctrl+G
    -- (control only, not command -- Cmd+G is Find Next).
    tell application "System Events"
        keystroke "g" using {control down}
        delay 0.5
        keystroke spec
        delay 0.4
        key code 36 -- Return
    end tell
    delay 0.4
end goToLine

on pressEscape()
    tell application "System Events" to key code 53
    delay 0.5
end pressEscape

on slowType(theText, perCharDelay)
    tell application "System Events"
        repeat with i from 1 to length of theText
            keystroke (character i of theText)
            delay perCharDelay
        end repeat
    end tell
end slowType

on toggleScreencastMode()
    tell application "System Events"
        keystroke "p" using {command down, shift down}
        delay 0.6
        keystroke "Developer: Toggle Screencast Mode"
        delay 0.8
        key code 36 -- Return
    end tell
    delay 1.0
end toggleScreencastMode

-- == Setup: focus VS Code and enable Screencast Mode ===========

tell application "Visual Studio Code"
    activate
end tell
delay 2.5

toggleScreencastMode()

-- == Scene 1: Hover on @spec citation (Cmd+K Cmd+I) ============

quickOpen("src/login.ts")
goToLine("6:12") -- inside `AUTH-001` in `// @spec AUTH-001, AUTH-002`

tell application "System Events"
    keystroke "k" using {command down}
    delay 0.2
    keystroke "i" using {command down}
end tell
delay 5.5 -- viewer reads the spec text, status, "Defined at ..."

pressEscape()

-- == Scene 2: Go to Definition (F12) ===========================

goToLine("6:12") -- re-anchor on the spec ID
tell application "System Events" to key code 111 -- F12
delay 5.0 -- spec file opens, target line highlighted

-- == Scene 3: Find All References (Shift+F12) ==================

-- After F12 the cursor lands at column 0 of the spec line, which
-- is BEFORE the bold AUTH-001 span. Shift+F12 only returns
-- references when the cursor is inside a spec ID, so we re-anchor
-- on column 11 (inside `**AUTH-001**`) first.
goToLine("11:11")
tell application "System Events" to key code 111 using {shift down}
delay 6.0 -- side panel populates with the citation list

pressEscape()
pressEscape() -- close the references panel if a second escape is needed

-- == Scene 4: Autocomplete spec IDs ============================

quickOpen("src/login.ts")
goToLine("32:99") -- past the last character on the last content line

tell application "System Events"
    key code 36 -- Return: new blank line below
end tell
delay 0.6

slowType("// @spec ", 0.12)
delay 3.5 -- popup shows AUTH-001 .. AUTH-005

slowType("AUTH-", 0.12)
delay 2.5 -- popup narrowed

pressEscape() -- dismiss popup so the next slowType doesn't insert from it

-- == Scene 5: Diagnostic for an undefined spec =================

slowType("999", 0.18) -- the line now reads `// @spec AUTH-999`
delay 3.0 -- LSP publishes the reverse-orphan diagnostic; squiggle appears

-- The new line is line 33 (we pressed Return from line 32). Put
-- the cursor inside `AUTH-999` so Cmd+K Cmd+I surfaces the
-- diagnostic.
goToLine("33:12")
tell application "System Events"
    keystroke "k" using {command down}
    delay 0.2
    keystroke "i" using {command down}
end tell
delay 6.0 -- "references a spec ID that is not defined ..."

pressEscape()

-- == Scene 6: Rename a spec across the workspace (F2) ==========

-- Rename from the source citation site so the cursor is already
-- in a renameable position. F2 from inside the AUTH-001 token
-- opens the rename input prefilled with `AUTH-001`.
goToLine("6:12") -- inside `AUTH-001` in `// @spec AUTH-001, AUTH-002`

tell application "System Events"
    key code 120 -- F2 (rename)
end tell
delay 1.6 -- rename input pops up preselected

slowType("AUTH-LOGIN-001", 0.10)
delay 1.2

tell application "System Events"
    key code 36 -- Return: apply the workspace edit
end tell
delay 5.5 -- spec file + every citation update simultaneously

-- Show the spec definition has been renamed.
quickOpen("docs/specs/auth-specs.md")
goToLine("11:1")
delay 4.0

-- Closing frame: back to the source file where the citation now
-- reads AUTH-LOGIN-001.
quickOpen("src/login.ts")
goToLine("6:1")
delay 4.0

-- == Teardown: disable Screencast Mode =========================

toggleScreencastMode()
