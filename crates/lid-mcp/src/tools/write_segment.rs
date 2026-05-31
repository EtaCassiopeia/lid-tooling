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

    {
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
        // Back-fill parent on each child.
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
}

pub async fn lid_update_segment(
    registry: &RepoRegistry,
    input: UpdateSegmentInput,
) -> Result<String, ErrorData> {
    let seg_id = SegmentId::parse(&input.segment_id)
        .map_err(|_| McpToolError::InvalidSegmentId(input.segment_id.clone()))
        .map_err(ErrorData::from)?;

    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let index_path = Path::new(&input.project_root)
        .join("docs")
        .join("arrows")
        .join("index.yaml");

    {
        let mut repo = handle.write().await;
        let seg = repo
            .index
            .arrows
            .get_mut(&seg_id)
            .ok_or_else(|| McpToolError::SegmentNotFound(input.segment_id.clone()))
            .map_err(ErrorData::from)?;

        if let Some(s) = &input.status {
            seg.status = parse_status(s).map_err(ErrorData::from)?;
        }
        if let Some(n) = &input.next {
            seg.next = if n.is_empty() { None } else { Some(n.clone()) };
        }
        if let Some(d) = &input.drift {
            seg.drift = if d.is_empty() { None } else { Some(d.clone()) };
        }

        let yaml = serde_yaml_ng::to_string(&repo.index)
            .map_err(|e| McpToolError::Yaml(e.to_string()))
            .map_err(ErrorData::from)?;
        drop(repo);
        atomic_write(&index_path, &yaml)
            .await
            .map_err(ErrorData::from)?;
    }

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
