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
| `lid_init` | Scaffold a new LID project — creates `index.yaml`, arrow doc stub, and intent files (`*-specs.md` with `prefix:` frontmatter, `*-design.md`) |
| `lid_discover` | Find and register a LID project; returns root and segment count |
| `lid_status` | Segment overview with spec counts |
| `lid_check` | Run coherence checks; returns all findings |
| `lid_list_segments` | List all segments with status and detail path |
| `lid_get_segment` | Full detail for one segment including spec lines |
| `lid_list_specs` | All spec lines; optional segment-ID prefix filter |
| `lid_find_spec_references` | Source-code locations citing `@spec SPEC-ID` |
| `lid_search` | Case-insensitive search across segment IDs, spec IDs, and text |
| `lid_add_segment` | Add a new segment to the arrow index and scaffold its intent files |
| `lid_update_segment` | Update status, next, drift, blocks, children, or parent on an existing segment |
| `lid_add_spec` | Append a new open spec line to a segment |
| `lid_update_spec_status` | Mark a spec open / implemented / deferred |
| `lid_update_spec_text` | Rewrite the requirement text of an existing spec line (preserves ID and status) |
| `lid_read_file` | Read any file within the project root (design docs, arrow docs, spec files) |
| `lid_append_to_design_doc` | Append markdown content to a segment's design doc; creates file with stub header if absent |

### `spec_prefix` parameter

Both `lid_init` and `lid_add_segment` accept an optional `spec_prefix` field. When omitted, the prefix is inferred: if a strict majority of the project's existing spec prefixes share the same namespace component (e.g. most start with `USH-`), that namespace is prepended to the uppercased segment name (`USH-BILLING`); otherwise the segment name is uppercased directly (`BILLING`).

---

## Typical agent workflow

```
1. lid_discover("/path/to/project")           → registers root, returns segment count
2. lid_status()                               → overview of all segments and spec coverage
3. lid_get_segment("auth")                    → full detail including spec lines
4. lid_read_file("docs/intent/auth/auth-design.md") → read design doc prose
5. lid_append_to_design_doc("auth", "## Decision\n...") → record intent in LLD
6. lid_add_spec("auth", "AUTH-042", "The system shall …") → add EARS spec line
7. lid_find_spec_references("AUTH-002")       → source files citing this spec
8. lid_update_spec_status("AUTH-002", "implemented")
9. lid_check()                                → verify no coherence regressions
```
