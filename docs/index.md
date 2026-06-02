---
title: Home
nav_order: 1
permalink: /
---

# LID Tooling
{: .no_toc }

Developer tools for the [LID (Linked-Intent Development)](https://linked-intent.dev) methodology — keep design intent permanently linked to running code.
{: .fs-6 .fw-300 }

[Get started]({{ site.baseurl }}/installation){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/EtaCassiopeia/lid-tooling){: .btn .fs-5 .mb-4 .mb-md-0 }

---

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
# 1. Install (macOS/Linux via Homebrew)
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

## How it works

```
docs/
├── arrows/
│   ├── index.yaml            # segment graph — statuses, dependencies
│   └── <segment>/*.md        # arrow detail docs (HLDs, LLDs)
└── intent/
    └── <segment>/
        ├── <segment>-specs.md    # spec lines  (- [ ] **ID**: text)
        └── <segment>-design.md   # design docs
```

Source code cites specs with a line comment:

```rust
// @spec AUTH-002
fn validate_session_expiry(session: &Session) -> bool { ... }
```

`lidc check` enforces that every `[x]` spec has at least one citation, every `@spec` reference points to a real spec ID, and the segment graph is acyclic — in your editor and in CI.

See [Project layout]({{ site.baseurl }}/project-layout) for the full schema reference.
