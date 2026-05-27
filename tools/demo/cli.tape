# VHS tape for the `lidc` CLI demo.
#
# Run from the repo root:
#
#   vhs tools/demo/cli.tape
#
# Output lands at tools/demo/cli.gif (and cli.mp4 if you swap the
# Output line). Make sure `lidc` is on PATH or in target/release/.

Output tools/demo/cli.gif

Set Theme "Catppuccin Mocha"
Set FontSize 14
Set Width 1200
Set Height 720
Set Padding 30
Set TypingSpeed 75ms
Set Shell zsh

Hide
Type "cd examples/sample-project"
Enter
Type "clear"
Enter
Show

# Let the viewer settle on the empty prompt for a moment.
Sleep 1.5s

# 1. Clean baseline — should read as "everything is fine."
Type "lidc check"
Sleep 600ms
Enter
Sleep 2.5s

# 2. Plant a reverse-orphan citation. The echo itself is quick,
#    but pause briefly so the viewer registers the change before
#    the check runs.
Type "echo '// @spec AUTH-999' >> src/login.ts"
Enter
Sleep 1s

Type "lidc check"
Sleep 600ms
Enter
Sleep 4s

# 3. JSON output for CI consumers — gives time to read the
#    summary object.
Type "lidc check --json | jq '.summary'"
Sleep 600ms
Enter
Sleep 4.5s

# 4. Filter to a single check.
Type "lidc check --only reverse-orphan"
Sleep 600ms
Enter
Sleep 4s

# 5. Fix the breakage. Final clean run gets a brief hold so the
#    demo ends on "no findings".
Type "sed -i '' -e '$d' src/login.ts && lidc check"
Sleep 600ms
Enter
Sleep 3s
