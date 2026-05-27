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
Set TypingSpeed 50ms
Set Shell zsh

Hide
Type "cd examples/sample-project"
Enter
Type "clear"
Enter
Show

Sleep 1s

# 1. Clean baseline
Type "lidc check"
Sleep 400ms
Enter
Sleep 2s

# 2. Plant a reverse-orphan citation
Type "echo '// @spec AUTH-999' >> src/login.ts"
Enter
Sleep 500ms

Type "lidc check"
Sleep 400ms
Enter
Sleep 3s

# 3. JSON output for CI consumers
Type "lidc check --json | jq '.summary'"
Sleep 400ms
Enter
Sleep 3s

# 4. Filter to a single check
Type "lidc check --only reverse-orphan"
Sleep 400ms
Enter
Sleep 3s

# 5. Fix the breakage
Type "sed -i '' -e '$d' src/login.ts && lidc check"
Sleep 400ms
Enter
Sleep 2s
