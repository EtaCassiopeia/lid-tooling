use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSegmentsInput {
    pub project_root: String,
}

#[derive(Debug, Serialize)]
struct SegmentEntry {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drift: Option<String>,
    blocks: Vec<String>,
    blocked_by: Vec<String>,
    children: Vec<String>,
    detail: String,
}

pub async fn lid_list_segments(
    registry: &RepoRegistry,
    input: ListSegmentsInput,
) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;
    let entries: Vec<SegmentEntry> = repo
        .index
        .arrows
        .iter()
        .map(|(id, seg)| SegmentEntry {
            id: id.to_string(),
            status: seg.status.to_string(),
            next: seg.next.clone(),
            drift: seg.drift.clone(),
            blocks: seg.blocks.iter().map(ToString::to_string).collect(),
            blocked_by: seg.blocked_by.iter().map(ToString::to_string).collect(),
            children: seg.children.iter().map(ToString::to_string).collect(),
            detail: seg.detail.to_string_lossy().into_owned(),
        })
        .collect();

    serde_json::to_string_pretty(&entries)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
