use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusInput {
    /// Absolute path to the LID project root (as returned by `lid_discover`).
    pub project_root: String,
}

#[derive(Debug, Serialize)]
struct SegmentSummary {
    id: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct StatusOutput {
    root: String,
    schema_version: u32,
    total_segments: usize,
    segments: Vec<SegmentSummary>,
    total_specs: usize,
    open_specs: usize,
    implemented_specs: usize,
    deferred_specs: usize,
}

pub async fn lid_status(registry: &RepoRegistry, input: StatusInput) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;

    let segments = repo
        .index
        .arrows
        .iter()
        .map(|(id, seg)| SegmentSummary {
            id: id.to_string(),
            status: format!("{:?}", seg.status),
        })
        .collect();

    let (mut open, mut implemented, mut deferred) = (0usize, 0usize, 0usize);
    for sf in &repo.specs {
        for sl in &sf.specs {
            match sl.status {
                lid_core::model::SpecStatus::Open => open += 1,
                lid_core::model::SpecStatus::Implemented => implemented += 1,
                lid_core::model::SpecStatus::Deferred => deferred += 1,
            }
        }
    }

    let out = StatusOutput {
        root: repo.root.to_string_lossy().into_owned(),
        schema_version: repo.index.schema_version,
        total_segments: repo.index.arrows.len(),
        segments,
        total_specs: open + implemented + deferred,
        open_specs: open,
        implemented_specs: implemented,
        deferred_specs: deferred,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
