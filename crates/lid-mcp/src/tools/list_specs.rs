use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSpecsInput {
    pub project_root: String,
    /// Optional segment ID prefix filter (e.g. "auth"). Case-insensitive.
    #[serde(default)]
    pub segment_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct SpecEntry {
    id: String,
    status: String,
    text: String,
    file: String,
}

pub async fn lid_list_specs(
    registry: &RepoRegistry,
    input: ListSpecsInput,
) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;
    let prefix = input.segment_id.as_deref().map(str::to_uppercase);

    let entries: Vec<SpecEntry> = repo
        .specs
        .iter()
        .flat_map(|sf| sf.specs.iter().map(|sl| (sf.path.clone(), sl.clone())))
        .filter(|(_, sl)| {
            prefix
                .as_deref()
                .is_none_or(|p| sl.id.as_ref().starts_with(p))
        })
        .map(|(path, sl)| SpecEntry {
            id: sl.id.to_string(),
            status: sl.status.to_string(),
            text: sl.text.clone(),
            file: path.to_string_lossy().into_owned(),
        })
        .collect();

    serde_json::to_string_pretty(&entries)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
