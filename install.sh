#!/usr/bin/env bash
# Install lidc and/or lid-mcp from the latest GitHub release.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash -s -- --mcp
#
# Flags:
#   --mcp        Also install lid-mcp (MCP server for AI agents)
#   --dir DIR    Install to DIR instead of /usr/local/bin
#   --version V  Install a specific version tag (e.g. v0.2.3)

set -euo pipefail

INSTALL_MCP=false
INSTALL_DIR="/usr/local/bin"
VERSION=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mcp)     INSTALL_MCP=true; shift ;;
    --dir)     INSTALL_DIR="$2"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    *)         echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

# ── Detect platform ──────────────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}-${ARCH}" in
  Darwin-arm64)  TARGET="aarch64-apple-darwin" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  Linux-x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  *) echo "Unsupported platform: ${OS}-${ARCH}" >&2; exit 1 ;;
esac

# ── Resolve version ──────────────────────────────────────────────────────────

if [[ -z "$VERSION" ]]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/EtaCassiopeia/lid-tooling/releases/latest" \
    | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
fi

echo "Installing lid-tooling ${VERSION} for ${TARGET}"

BASE_URL="https://github.com/EtaCassiopeia/lid-tooling/releases/download/${VERSION}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

install_bin() {
  local name="$1"
  echo "  → ${name}"
  curl -fsSL "${BASE_URL}/${name}-${TARGET}" -o "${TMP}/${name}"
  chmod +x "${TMP}/${name}"
  if [[ -w "$INSTALL_DIR" ]]; then
    mv "${TMP}/${name}" "${INSTALL_DIR}/${name}"
  else
    sudo mv "${TMP}/${name}" "${INSTALL_DIR}/${name}"
  fi
}

mkdir -p "$INSTALL_DIR"
install_bin "lidc"
$INSTALL_MCP && install_bin "lid-mcp"

echo ""
echo "Done!  Try:  lidc --version"
$INSTALL_MCP && echo "         and:  lid-mcp --help"
