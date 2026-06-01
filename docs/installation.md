---
title: Installation
nav_order: 2
---

# Installation

{: .no_toc }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## VS Code extension

### From the Marketplace (recommended)

Search **LID** in the VS Code Extensions panel and click **Install** — this works even behind a corporate proxy.

Or install directly from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=lid-tools.lid).

> **Note:** The CLI command `code --install-extension lid-tools.lid` may fail with a certificate error on machines with SSL inspection. Use the Extensions panel or install from a VSIX instead (see below).

`lid-lsp` is bundled — nothing else to install.

### From a GitHub release

Download `lid-vscode-<platform>.vsix` from the [latest release](https://github.com/EtaCassiopeia/lid-tooling/releases/latest), then:

1. Open the Extensions panel → **⋯ → Install from VSIX…**
2. Select the downloaded `.vsix`

### VS Code-compatible editors

The LID extension uses only standard VS Code API surface and is published to **Open VSX** — the registry used by most VS Code forks. It works out of the box in Cursor, Windsurf, and VSCodium with no extra steps.

| Editor | Install method |
|--------|---------------|
| **Cursor** | Search **LID** in the Extensions panel |
| **Windsurf** | Search **LID** in the Extensions panel |
| **VSCodium** | Search **LID** in the Extensions panel (Open VSX) |
| **Positron** | Download `lid-vscode-<platform>.vsix` from the [latest release](https://github.com/EtaCassiopeia/lid-tooling/releases/latest) → **Extensions: Install from VSIX** |

`lid-lsp` is bundled — nothing else to install.

> **Note:** Cursor and Windsurf also support `lid-mcp` natively via their built-in MCP integration. See [MCP Server](mcp.md) for per-client configuration snippets.

---

## IntelliJ IDEA / JetBrains IDEs

The plugin works in all JetBrains IDEs: IntelliJ IDEA, GoLand, PyCharm, RustRover, WebStorm, and more.

### From the JetBrains Marketplace (recommended)

<script src="https://plugins.jetbrains.com/assets/scripts/mp-widget.js"></script>
<div id="jb-install-btn"></div>
<script>
  MarketplaceWidget.setupMarketplaceWidget('card', 32047, "#jb-install-btn");
</script>

Or open **Settings → Plugins → Marketplace**, search **LID**, and click **Install**.

Or install directly from the [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/32047-lid--linked-intent-development).

`lid-lsp` is bundled — nothing else to install.

### From a GitHub release (install from disk)

Download `lid-intellij-<version>.zip` from the [latest release](https://github.com/EtaCassiopeia/lid-tooling/releases/latest), then:

1. Open **Settings → Plugins → ⚙ → Install Plugin from Disk…**
2. Select the downloaded `.zip`
3. Restart when prompted

### From source

Requires JDK 21+.

```sh
# Build lid-lsp first
cargo build --release --bin lid-lsp
cp target/release/lid-lsp extensions/intellij/server/lid-lsp

# Build the plugin zip
cd extensions/intellij
./gradlew buildPlugin
# → build/distributions/lid-intellij-<version>.zip

# Or launch a sandboxed IDE with the plugin pre-installed
./gradlew runIde
```

---

## CLI + MCP server

### macOS / Linux — Homebrew

```sh
brew tap EtaCassiopeia/lid
brew install lid-tooling
```

Installs `lidc`, `lid-mcp`, and `lid-lsp` on your `$PATH`.

### macOS / Linux — curl

Supported platforms: macOS (Apple Silicon, Intel), Linux x86_64, Linux ARM64 (Graviton, Raspberry Pi, Ampere).

```sh
# lidc only
curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash

# lidc + lid-mcp
curl -fsSL https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh | bash -s -- --mcp
```

### Windows — PowerShell

```powershell
# lidc only
irm https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.ps1 | iex

# lidc + lid-mcp
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.ps1))) -Mcp
```

### From source (Cargo)

```sh
cargo install --git https://github.com/EtaCassiopeia/lid-tooling lidc lid-mcp
```
