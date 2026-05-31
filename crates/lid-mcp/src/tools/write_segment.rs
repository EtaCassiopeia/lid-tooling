use std::path::Path;

use lid_core::model::{Segment, SegmentId, Status};
use lid_core::scaffold;
use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::error::McpToolError;
use crate::registry::RepoRegistry;
use crate::tools::write_spec::WriteResult;

// ── lid_add_segment ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddSegmentInput {
    pub project_root: String,
    /// New segment ID (e.g. "auth").
    pub segment_id: String,
    /// Status string: "UNMAPPED", "MAPPED", etc.
    pub status: String,
    /// Path to the detail document, relative to `docs/arrows/`.
    pub detail: String,
    /// Segment IDs this segment blocks (optional).
    #[serde(default)]
    pub blocks: Vec<String>,
    /// Child segment IDs (optional).
    #[serde(default)]
    pub children: Vec<String>,
    /// Spec-ID prefix (e.g. "MYAPP-AUTH"). If omitted, derived from the
    /// project's existing prefixes or the segment name.
    #[serde(default)]
    pub spec_prefix: Option<String>,
}

#[allow(clippy::too_many_lines)]
pub async fn lid_add_segment(
    registry: &RepoRegistry,
    input: AddSegmentInput,
) -> Result<String, ErrorData> {
    let seg_id = SegmentId::parse(&input.segment_id)
        .map_err(|_| McpToolError::InvalidSegmentId(input.segment_id.clone()))
        .map_err(ErrorData::from)?;
    let status = parse_status(&input.status).map_err(ErrorData::from)?;

    let blocks: Vec<SegmentId> = input
        .blocks
        .iter()
        .map(|s| {
            SegmentId::parse(s)
                .map_err(|_| McpToolError::InvalidSegmentId(s.clone()))
                .map_err(ErrorData::from)
        })
        .collect::<Result<_, _>>()?;

    let children: Vec<SegmentId> = input
        .children
        .iter()
        .map(|s| {
            SegmentId::parse(s)
                .map_err(|_| McpToolError::InvalidSegmentId(s.clone()))
                .map_err(ErrorData::from)
        })
        .collect::<Result<_, _>>()?;

    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    // Resolve spec prefix: explicit → infer from existing prefixes → derive from name.
    let spec_prefix = {
        let repo = handle.read().await;
        if repo.index.arrows.contains_key(&seg_id) {
            return Err(ErrorData::from(McpToolError::DuplicateSegment(
                input.segment_id.clone(),
            )));
        }
        input.spec_prefix.clone().unwrap_or_else(|| {
            let existing: Vec<&str> = repo
                .specs
                .iter()
                .filter_map(|sf| sf.prefix.as_deref())
                .collect();
            scaffold::suggest_prefix(&input.segment_id, &existing)
        })
    };

    let index_path = Path::new(&input.project_root)
        .join("docs")
        .join("arrows")
        .join("index.yaml");

    // Text-append the new segment so comments, blank lines, and formatting in
    // the existing file are preserved. Children back-fill (setting parent: on
    // existing entries) still requires a round-trip, but skipping it when
    // there are no children is the common case.
    {
        let raw = tokio::fs::read_to_string(&index_path)
            .await
            .map_err(McpToolError::Io)
            .map_err(ErrorData::from)?;
        let block = build_segment_block(
            &input.segment_id,
            status.as_str(),
            &input.detail,
            &blocks,
            &children,
        );
        let updated = insert_into_arrows(&raw, &block);
        atomic_write(&index_path, &updated)
            .await
            .map_err(ErrorData::from)?;
    }

    if children.is_empty() {
        // Common case: no children to back-fill. Update in-memory state only.
        let mut repo = handle.write().await;
        let new_seg = Segment {
            status,
            detail: std::path::PathBuf::from(&input.detail),
            sampled: None,
            audited: None,
            audited_sha: None,
            blocks: blocks.clone(),
            blocked_by: Vec::new(),
            next: None,
            drift: None,
            merged_into: None,
            children: children.clone(),
            parent: None,
        };
        repo.index.arrows.insert(seg_id.clone(), new_seg);
        drop(repo);
    } else {
        // Back-fill parent on existing children. Requires modifying those entries,
        // so we do a full round-trip (model now has skip_serializing_if to limit damage).
        let mut repo = handle.write().await;
        for child in &children {
            if let Some(entry) = repo.index.arrows.get_mut(child) {
                entry.parent = Some(seg_id.clone());
            }
        }
        let yaml = serde_yaml_ng::to_string(&repo.index)
            .map_err(|e| McpToolError::Yaml(e.to_string()))
            .map_err(ErrorData::from)?;
        drop(repo);
        atomic_write(&index_path, &yaml)
            .await
            .map_err(ErrorData::from)?;
    }

    let root = Path::new(&input.project_root);
    tokio::task::spawn_blocking({
        let root = root.to_path_buf();
        let seg = input.segment_id.clone();
        let detail = input.detail.clone();
        let prefix = spec_prefix.clone();
        move || -> Result<(), McpToolError> {
            scaffold::scaffold_arrow_doc(&root, &detail, &seg).map_err(McpToolError::Io)?;
            scaffold::scaffold_intent_dir(&root, &seg, &prefix).map_err(McpToolError::Io)?;
            Ok(())
        }
    })
    .await
    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
    .map_err(ErrorData::from)?;

    let rediscover_error = registry.rediscover(root).await;
    let out = WriteResult {
        action: format!("added segment {} (prefix: {spec_prefix})", input.segment_id),
        rediscover_error,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

// ── lid_update_segment ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateSegmentInput {
    pub project_root: String,
    pub segment_id: String,
    /// When present, update the status.
    #[serde(default)]
    pub status: Option<String>,
    /// When present, set or clear the `next` field (empty string → clear).
    #[serde(default)]
    pub next: Option<String>,
    /// When present, set or clear the `drift` field (empty string → clear).
    #[serde(default)]
    pub drift: Option<String>,
    /// When present, replace the `blocks` array (empty list → remove field).
    #[serde(default)]
    pub blocks: Option<Vec<String>>,
    /// When present, replace the `children` array (empty list → remove field).
    #[serde(default)]
    pub children: Option<Vec<String>>,
    /// When present, set or clear the `parent` field (empty string → clear).
    #[serde(default)]
    pub parent: Option<String>,
}

pub async fn lid_update_segment(
    registry: &RepoRegistry,
    input: UpdateSegmentInput,
) -> Result<String, ErrorData> {
    let seg_id = SegmentId::parse(&input.segment_id)
        .map_err(|_| McpToolError::InvalidSegmentId(input.segment_id.clone()))
        .map_err(ErrorData::from)?;

    // Validate blocks/children/parent segment IDs up front.
    if let Some(ref b) = input.blocks {
        for s in b {
            SegmentId::parse(s)
                .map_err(|_| McpToolError::InvalidSegmentId(s.clone()))
                .map_err(ErrorData::from)?;
        }
    }
    if let Some(ref c) = input.children {
        for s in c {
            SegmentId::parse(s)
                .map_err(|_| McpToolError::InvalidSegmentId(s.clone()))
                .map_err(ErrorData::from)?;
        }
    }

    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    // Verify the segment exists.
    {
        let repo = handle.read().await;
        if !repo.index.arrows.contains_key(&seg_id) {
            return Err(ErrorData::from(McpToolError::SegmentNotFound(
                input.segment_id.clone(),
            )));
        }
    }

    let index_path = Path::new(&input.project_root)
        .join("docs")
        .join("arrows")
        .join("index.yaml");

    let mut content = tokio::fs::read_to_string(&index_path)
        .await
        .map_err(McpToolError::Io)
        .map_err(ErrorData::from)?;

    let seg = input.segment_id.as_str();

    if let Some(s) = &input.status {
        let status = parse_status(s).map_err(ErrorData::from)?;
        content = patch_segment_scalar(&content, seg, "status", Some(status.as_str()));
    }
    if let Some(n) = &input.next {
        let val = if n.is_empty() { None } else { Some(n.as_str()) };
        content = patch_segment_scalar(&content, seg, "next", val);
    }
    if let Some(d) = &input.drift {
        let val = if d.is_empty() { None } else { Some(d.as_str()) };
        content = patch_segment_scalar(&content, seg, "drift", val);
    }
    if let Some(b) = &input.blocks {
        let items: Vec<&str> = b.iter().map(String::as_str).collect();
        content = patch_segment_inline_array(&content, seg, "blocks", &items);
    }
    if let Some(c) = &input.children {
        let items: Vec<&str> = c.iter().map(String::as_str).collect();
        content = patch_segment_inline_array(&content, seg, "children", &items);
    }
    if let Some(p) = &input.parent {
        let val = if p.is_empty() { None } else { Some(p.as_str()) };
        content = patch_segment_scalar(&content, seg, "parent", val);
    }

    atomic_write(&index_path, &content)
        .await
        .map_err(ErrorData::from)?;

    let rediscover_error = registry.rediscover(Path::new(&input.project_root)).await;
    let out = WriteResult {
        action: format!("updated segment {}", input.segment_id),
        rediscover_error,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_status(s: &str) -> Result<Status, McpToolError> {
    match s.to_uppercase().as_str() {
        "UNMAPPED" => Ok(Status::Unmapped),
        "MAPPED" => Ok(Status::Mapped),
        "AUDITED" => Ok(Status::Audited),
        "OK" => Ok(Status::Ok),
        "PARTIAL" => Ok(Status::Partial),
        "BROKEN" => Ok(Status::Broken),
        "STALE" => Ok(Status::Stale),
        "OBSOLETE" => Ok(Status::Obsolete),
        "MERGED" => Ok(Status::Merged),
        other => Err(McpToolError::InvalidStatus(other.to_owned())),
    }
}

async fn atomic_write(path: &Path, content: &str) -> Result<(), McpToolError> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, content).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}

/// Build a two-space-indented YAML block for a new segment (no trailing newline).
fn build_segment_block(
    seg_id: &str,
    status: &str,
    detail: &str,
    blocks: &[SegmentId],
    children: &[SegmentId],
) -> String {
    let mut lines = vec![format!("  {seg_id}:")];
    lines.push(format!("    status: {status}"));
    // Quote detail if it contains YAML-special characters.
    if detail.contains([':', '#', '[', ']', '{', '}', '|', '>', '&', '*', '!', ',']) {
        lines.push(format!("    detail: {detail:?}"));
    } else {
        lines.push(format!("    detail: {detail}"));
    }
    if !blocks.is_empty() {
        let list = blocks
            .iter()
            .map(SegmentId::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("    blocks: [{list}]"));
    }
    if !children.is_empty() {
        let list = children
            .iter()
            .map(SegmentId::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("    children: [{list}]"));
    }
    lines.join("\n")
}

/// Insert `block` into the `arrows:` section of `content`, immediately before
/// the first top-level key that follows `arrows:`. Appends at end of file when
/// `arrows:` is the last top-level section.
fn insert_into_arrows(content: &str, block: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut insert_at = lines.len();
    let mut in_arrows = false;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("arrows:") {
            in_arrows = true;
            continue;
        }
        if in_arrows && !line.is_empty() && !line.starts_with(|c: char| c.is_whitespace()) {
            insert_at = i;
            break;
        }
    }
    let mut result: Vec<&str> = lines[..insert_at].to_vec();
    result.push(block);
    result.extend_from_slice(&lines[insert_at..]);
    result.join("\n")
}

/// Patch a scalar field on an existing segment in raw `index.yaml` text.
/// `value = Some(v)` adds or replaces the field; `value = None` removes it.
/// All other content (comments, dates, inline arrays) is untouched.
pub(crate) fn patch_segment_scalar(
    content: &str,
    seg_id: &str,
    field: &str,
    value: Option<&str>,
) -> String {
    let seg_key = format!("  {seg_id}:");
    let field_prefix = format!("    {field}: ");

    let lines: Vec<&str> = content.split('\n').collect();

    let Some(seg_start) = lines.iter().position(|&l| l == seg_key.as_str()) else {
        return content.to_owned();
    };

    let mut field_idx: Option<usize> = None;
    let mut last_field_idx = seg_start;

    for (i, line) in lines.iter().enumerate().skip(seg_start + 1) {
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("    ") {
            break;
        }
        last_field_idx = i;
        if line.starts_with(&field_prefix) {
            field_idx = Some(i);
        }
    }

    let mut out: Vec<String> = lines.iter().map(ToString::to_string).collect();

    match (field_idx, value) {
        (Some(idx), Some(val)) => {
            out[idx] = format!("    {field}: {}", yaml_scalar(val));
        }
        (Some(idx), None) => {
            out.remove(idx);
        }
        (None, Some(val)) => {
            out.insert(
                last_field_idx + 1,
                format!("    {field}: {}", yaml_scalar(val)),
            );
        }
        (None, None) => {}
    }

    out.join("\n")
}

/// Patch an inline-array field on an existing segment in raw `index.yaml` text.
/// An empty `items` slice removes the field; non-empty replaces or inserts it
/// as `    {field}: [{item1}, {item2}, ...]`.
pub(crate) fn patch_segment_inline_array(
    content: &str,
    seg_id: &str,
    field: &str,
    items: &[&str],
) -> String {
    let seg_key = format!("  {seg_id}:");
    let field_key = format!("    {field}:");

    let lines: Vec<&str> = content.split('\n').collect();

    let Some(seg_start) = lines.iter().position(|&l| l == seg_key.as_str()) else {
        return content.to_owned();
    };

    let mut field_idx: Option<usize> = None;
    let mut last_field_idx = seg_start;

    for (i, line) in lines.iter().enumerate().skip(seg_start + 1) {
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("    ") {
            break;
        }
        last_field_idx = i;
        if line.starts_with(&field_key) {
            field_idx = Some(i);
        }
    }

    let mut out: Vec<String> = lines.iter().map(ToString::to_string).collect();

    if items.is_empty() {
        if let Some(idx) = field_idx {
            out.remove(idx);
        }
    } else {
        let list = items.join(", ");
        let new_line = format!("    {field}: [{list}]");
        match field_idx {
            Some(idx) => out[idx] = new_line,
            None => out.insert(last_field_idx + 1, new_line),
        }
    }

    out.join("\n")
}

/// Serialize a scalar value for inline YAML. Wraps in double quotes if the
/// value contains YAML-special characters or whitespace that could mislead
/// a parser.
fn yaml_scalar(s: &str) -> String {
    let needs_quotes = s.is_empty()
        || s.contains([
            ':', '#', '[', ']', '{', '}', '|', '>', '&', '*', '!', ',', '\'', '"', '\n',
        ])
        || s.starts_with('-')
        || s.starts_with('?');
    if needs_quotes {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        s.to_owned()
    }
}
