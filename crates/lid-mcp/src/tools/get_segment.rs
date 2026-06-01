use lid_core::model::DecisionScope;
use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::McpToolError;
use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSegmentInput {
    pub project_root: String,
    pub segment_id: String,
}

#[derive(Debug, Serialize)]
struct SpecSummary {
    id: String,
    status: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct DecisionSummary {
    path: String,
    title: String,
}

#[derive(Debug, Serialize)]
struct SegmentDetail {
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
    specs: Vec<SpecSummary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    decision_docs: Vec<DecisionSummary>,
}

pub async fn lid_get_segment(
    registry: &RepoRegistry,
    input: GetSegmentInput,
) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;
    let seg_id = &input.segment_id;

    let seg = repo
        .index
        .arrows
        .iter()
        .find(|(id, _)| id.as_ref() == seg_id)
        .map(|(_, s)| s)
        .ok_or_else(|| McpToolError::SegmentNotFound(seg_id.clone()))
        .map_err(ErrorData::from)?;

    // Collect specs whose ID starts with the segment prefix (e.g. "auth" → "AUTH-*").
    let prefix = seg_id.to_uppercase();
    let specs: Vec<SpecSummary> = repo
        .specs
        .iter()
        .flat_map(|sf| sf.specs.iter())
        .filter(|sl| sl.id.as_ref().starts_with(&prefix))
        .map(|sl| SpecSummary {
            id: sl.id.to_string(),
            status: sl.status.to_string(),
            text: sl.text.clone(),
        })
        .collect();

    let segment_lower = seg_id.to_lowercase();
    let decision_docs: Vec<DecisionSummary> = repo
        .decision_docs
        .iter()
        .filter(
            |d| matches!(&d.scope, DecisionScope::Node { segment } if segment == &segment_lower),
        )
        .map(|d| DecisionSummary {
            path: d.path.to_string_lossy().into_owned(),
            title: d.title.clone(),
        })
        .collect();

    let detail = SegmentDetail {
        id: seg_id.clone(),
        status: seg.status.to_string(),
        next: seg.next.clone(),
        drift: seg.drift.clone(),
        blocks: seg.blocks.iter().map(ToString::to_string).collect(),
        blocked_by: seg.blocked_by.iter().map(ToString::to_string).collect(),
        children: seg.children.iter().map(ToString::to_string).collect(),
        detail: seg.detail.to_string_lossy().into_owned(),
        specs,
        decision_docs,
    };
    serde_json::to_string_pretty(&detail)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
