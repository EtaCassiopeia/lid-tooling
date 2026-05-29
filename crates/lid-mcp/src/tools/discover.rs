use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscoverInput {
    /// Absolute path to (or within) the LID project root.
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DiscoverOutput {
    pub root: String,
    pub schema_version: u32,
    pub segment_count: usize,
}

pub async fn lid_discover(
    registry: &RepoRegistry,
    input: DiscoverInput,
) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.path)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;
    let out = DiscoverOutput {
        root: repo.root.to_string_lossy().into_owned(),
        schema_version: repo.index.schema_version,
        segment_count: repo.index.arrows.len(),
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
