#!/usr/bin/env bash
#
# Drive Visual Studio Code through the LID extension demo and
# capture the screen with ffmpeg.
#
# Usage:
#
#   tools/demo/record-vscode.sh           # records to vscode.mp4
#   tools/demo/record-vscode.sh out.mp4   # records to a custom path
#
# Prereqs (one-time):
#   * Grant Accessibility + Screen Recording to whatever shell
#     runs this (likely Warp or Terminal). System Settings -->
#     Privacy & Security -> Accessibility / Screen Recording.
#   * `ffmpeg` on PATH (e.g. `brew install ffmpeg`).
#   * `code` on PATH (VS Code's CLI; install via Command Palette
#     -> "Shell Command: Install 'code' command in PATH").
#   * The Rust binaries built and `lid.serverPath` configured.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
output="${1:-${repo_root}/tools/demo/vscode.mp4}"
duration="${DEMO_SECONDS:-95}"
warmup="${DEMO_WARMUP:-6}"

cd "$repo_root"

echo "==> Resetting examples/sample-project to a clean baseline"
( cd examples/sample-project && git checkout . >/dev/null 2>&1 )

echo "==> Opening the sample project in VS Code"
code "${repo_root}/examples/sample-project"

echo "==> Bringing VS Code to the front (osascript)"
osascript -e 'tell application "Visual Studio Code" to activate'

echo "==> Warm-up: ${warmup}s for VS Code + LSP to settle."
echo "    If VS Code isn't the focused window, switch to it NOW."
echo "    The recorder starts after this countdown."
for ((i = warmup; i > 0; i--)); do
    printf "    %ss...\r" "$i"
    sleep 1
done
echo

echo "==> Starting ffmpeg (duration: ${duration}s, output: ${output})"
ffmpeg \
    -y -hide_banner -loglevel error \
    -f avfoundation -framerate 30 -pixel_format uyvy422 \
    -i "2:none" \
    -t "$duration" \
    -c:v libx264 -pix_fmt yuv420p -crf 23 \
    "$output" >/tmp/ffmpeg-vscode-demo.log 2>&1 &
ffmpeg_pid=$!

sleep 2 # give the recorder a moment to grab the screen

echo "==> Running the AppleScript demo (don't touch keyboard/mouse)"
osascript "${repo_root}/tools/demo/vscode-demo.applescript"

echo "==> AppleScript done. Waiting for ffmpeg to finish (timer expires)..."
wait "$ffmpeg_pid"

echo "==> Resetting sample project (the rename and added line leave it dirty)"
( cd examples/sample-project && git checkout . >/dev/null 2>&1 )

echo "==> Output:"
ls -lh "$output"
echo
echo "Open it with:  open ${output}"
