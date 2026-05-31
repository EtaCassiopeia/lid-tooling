---
title: MCP Server
nav_order: 6
---

# `lid-mcp` MCP Server

{: .no_toc }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

`lid-mcp` is a stdio MCP server — AI agents call its tools to inspect and modify a LID project with the same integrity guarantees as the LSP and CLI.

---

## Configuration

### Claude Desktop (macOS)

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "lid": { "command": "lid-mcp" }
  }
}
```

### Cursor / VS Code / other MCP clients

Add `.mcp.json` at your project root:

```json
{
  "servers": {
    "lid": { "type": "stdio", "command": "lid-mcp" }
  }
}
```

---

## Tools

Always call `lid_discover` first to register the project root, then use any other tool.

| Tool | Description |
|------|-------------|
| `lid_init` | Scaffold a new LID project at a path |
| `lid_discover` | Find and register a LID project; returns root and segment count |
| `lid_status` | Segment overview with spec counts |
| `lid_check` | Run coherence checks; returns all findings |
| `lid_list_segments` | List all segments with status and detail path |
| `lid_get_segment` | Full detail for one segment including spec lines |
| `lid_list_specs` | All spec lines; optional segment-ID prefix filter |
| `lid_find_spec_references` | Source-code locations citing `@spec SPEC-ID` |
| `lid_search` | Case-insensitive search across segment IDs, spec IDs, and text |
| `lid_add_segment` | Add a new segment to the arrow index |
| `lid_update_segment` | Update status, next, or drift on an existing segment |
| `lid_add_spec` | Append a new open spec line to a segment |
| `lid_update_spec_status` | Mark a spec open / implemented / deferred |

---

## Typical agent workflow

```
1. lid_discover("/path/to/project")   → registers root, returns segment count
2. lid_status()                        → overview of all segments and spec coverage
3. lid_get_segment("auth")             → full detail including spec lines
4. lid_find_spec_references("AUTH-002")→ source files citing this spec
5. lid_update_spec_status("AUTH-002", "implemented")
6. lid_check()                         → verify no coherence regressions
```
