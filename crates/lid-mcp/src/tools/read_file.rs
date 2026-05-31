use std::path::Path;

use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::error::McpToolError;
use crate::registry::RepoRegistry;
use crate::tools::write_spec::WriteResult;

// ── lid_read_file ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadFileInput {
    pub project_root: String,
    /// Path to the file, relative to `project_root` (e.g.
    /// `"docs/intent/auth/auth-design.md"`). Must not contain `..`.
    pub path: String,
}

pub async fn lid_read_file(
    _registry: &RepoRegistry,
    input: ReadFileInput,
) -> Result<String, ErrorData> {
    // Reject path traversal before touching the filesystem.
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

// ── lid_append_to_design_doc ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppendToDesignDocInput {
    pub project_root: String,
    /// The segment whose design doc to append to (e.g. `"auth"`).
    pub segment_id: String,
    /// Markdown content to append. A blank line is inserted before the content
    /// when the file already exists so sections are separated.
    pub content: String,
}

pub async fn lid_append_to_design_doc(
    registry: &RepoRegistry,
    input: AppendToDesignDocInput,
) -> Result<String, ErrorData> {
    let seg = input.segment_id.to_lowercase();
    let design_path = Path::new(&input.project_root)
        .join("docs")
        .join("intent")
        .join(&seg)
        .join(format!("{seg}-design.md"));

    if design_path.exists() {
        let mut existing = tokio::fs::read_to_string(&design_path)
            .await
            .map_err(McpToolError::Io)
            .map_err(ErrorData::from)?;
        if !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push('\n');
        existing.push_str(&input.content);
        if !existing.ends_with('\n') {
            existing.push('\n');
        }
        atomic_write(&design_path, &existing)
            .await
            .map_err(ErrorData::from)?;
    } else {
        if let Some(parent) = design_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(McpToolError::Io)
                .map_err(ErrorData::from)?;
        }
        let header = format!(
            "# {seg} design\n\n## Overview\n\n<!-- Describe the design for {seg} here. -->\n\n"
        );
        let body = format!("{}{}\n", header, input.content);
        atomic_write(&design_path, &body)
            .await
            .map_err(ErrorData::from)?;
    }

    let rediscover_error = registry.rediscover(Path::new(&input.project_root)).await;

    let out = WriteResult {
        action: format!("appended to {}", design_path.display()),
        rediscover_error,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

// ── helpers ───────────────────────────────────────────────────────────────────

async fn atomic_write(path: &Path, content: &str) -> Result<(), McpToolError> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, content).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}
