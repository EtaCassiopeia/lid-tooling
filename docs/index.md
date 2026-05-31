---
title: Home
nav_order: 1
permalink: /
---

# LID Tooling

<p align="center">
  <img src="https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/extensions/vscode/images/lid_tooling_logo.svg" alt="lid-tools logo" width="640"/>
</p>

Developer tools for the [LID (Linked-Intent Development)](https://github.com/jszmajda/lid) methodology — keep design intent permanently linked to running code.

LID answers one recurring problem: *you know what the code does, but you've lost track of why it exists and whether it still does what the design said it should.* This tooling enforces the link between requirements, design docs, and test citations — in your editor, your CI pipeline, and your AI workflow.

---

## Tools

| Tool | What it does |
|------|-------------|
| **[VS Code extension]({{ site.baseurl }}/vscode)** | LSP diagnostics, hover, go-to-definition, rename, completion, visual Intent Navigator |
| **[IntelliJ IDEA plugin]({{ site.baseurl }}/intellij)** | LSP diagnostics, hover, go-to-definition, find references, rename, completion, and visual Intent Navigator for all JetBrains IDEs |
| **[`lidc` CLI]({{ site.baseurl }}/cli)** | `lidc check` for CI, `lidc init` to scaffold new projects, `lidc status` for a quick summary |
| **[`lid-mcp` MCP server]({{ site.baseurl }}/mcp)** | 13 tools so AI agents can read and write LID projects with full integrity guarantees |

---

## Quick start

```sh
# 1. Install lidc (macOS/Linux via Homebrew)
brew tap EtaCassiopeia/lid && brew install lid-tooling

# 2. Scaffold a new LID project
lidc init

# 3. Run coherence checks
lidc check

# 4. See spec coverage
lidc status
```

See [Installation]({{ site.baseurl }}/installation) for all platforms and install methods.

---

## Project layout

```
docs/
├── arrows/
│   ├── index.yaml            # segment graph
│   └── <segment>/*.md        # arrow detail docs (HLDs, LLDs)
└── intent/
    └── <segment>/
        ├── <segment>-specs.md
        └── <segment>-design.md
```

See [Project layout]({{ site.baseurl }}/project-layout) for the full schema reference and file formats.
