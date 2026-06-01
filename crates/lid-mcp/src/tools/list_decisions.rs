use lid_core::model::DecisionScope;
use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

// ── lid_list_decisions ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDecisionsInput {
    pub project_root: String,
    /// Optional filter: `"project"` for project-level decisions only,
    /// `"node"` for per-node decisions only.  Omit to return both.
    #[serde(default)]
    pub scope_filter: Option<String>,
}

#[derive(Debug, Serialize)]
struct DecisionEntry {
    path: String,
    scope: DecisionScopeOutput,
    title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum DecisionScopeOutput {
    Project,
    Node { segment: String },
}

pub async fn lid_list_decisions(
    registry: &RepoRegistry,
    input: ListDecisionsInput,
) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;

    let entries: Vec<DecisionEntry> = repo
        .decision_docs
        .iter()
        .filter(|d| match input.scope_filter.as_deref() {
            Some("project") => matches!(d.scope, DecisionScope::Project),
            Some("node") => matches!(d.scope, DecisionScope::Node { .. }),
            _ => true,
        })
        .map(|d| DecisionEntry {
            path: d.path.to_string_lossy().into_owned(),
            scope: match &d.scope {
                DecisionScope::Project => DecisionScopeOutput::Project,
                DecisionScope::Node { segment } => DecisionScopeOutput::Node {
                    segment: segment.clone(),
                },
            },
            title: d.title.clone(),
        })
        .collect();

    serde_json::to_string_pretty(&entries)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

// ── lid_get_decision ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDecisionInput {
    pub project_root: String,
    /// Path to the decision doc relative to `project_root`
    /// (e.g. `"docs/decisions/arch.md"` or
    /// `"docs/intent/auth/decisions/token-format.md"`).
    /// Must not contain `..`.
    pub path: String,
}

pub async fn lid_get_decision(
    _registry: &RepoRegistry,
    input: GetDecisionInput,
) -> Result<String, ErrorData> {
    use std::path::Path;

    use crate::error::McpToolError;

    if Path::new(&input.path)
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(ErrorData::from(McpToolError::InvalidSpecId(format!(
            "path '{}' must not contain '..'",
            input.path
        ))));
    }

    let file_path = Path::new(&input.project_root).join(&input.path);
    tokio::fs::read_to_string(&file_path)
        .await
        .map_err(McpToolError::Io)
        .map_err(ErrorData::from)
}
