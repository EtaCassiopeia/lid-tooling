use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindRefsInput {
    pub project_root: String,
    /// The spec ID to find references for (e.g. "AUTH-UI-001").
    pub spec_id: String,
}

#[derive(Debug, Serialize)]
struct CitationEntry {
    file: String,
    line: usize,
    kind: String,
}

pub async fn lid_find_spec_references(
    registry: &RepoRegistry,
    input: FindRefsInput,
) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;
    let target = input.spec_id.to_uppercase();

    let refs: Vec<CitationEntry> = repo
        .citations
        .iter()
        .filter(|c| c.id.as_ref().eq_ignore_ascii_case(&target))
        .map(|c| CitationEntry {
            file: c.file.to_string_lossy().into_owned(),
            line: c.line,
            kind: c.kind.to_string(),
        })
        .collect();

    serde_json::to_string_pretty(&refs).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
