use std::path::Path;

use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::McpToolError;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InitInput {
    /// Absolute path to the directory where the LID project should be created.
    pub path: String,
    /// Name of the first segment (default: "core"). Lowercase letters, digits,
    /// and hyphens only.
    #[serde(default = "default_segment")]
    pub segment: String,
}

fn default_segment() -> String {
    "core".to_owned()
}

#[derive(Debug, Serialize)]
pub struct InitOutput {
    pub root: String,
    pub segment: String,
    pub files_created: Vec<String>,
}

pub async fn lid_init(input: InitInput) -> Result<String, ErrorData> {
    let path = input.path.clone();
    let segment = input.segment.clone();
    tokio::task::spawn_blocking(move || init_blocking(&path, &segment))
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(ErrorData::from)
        .and_then(|out| {
            serde_json::to_string_pretty(&out)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))
        })
}

fn init_blocking(path: &str, segment: &str) -> Result<InitOutput, McpToolError> {
    if !is_valid_segment(segment) {
        return Err(McpToolError::InvalidSegmentId(format!(
            "'{segment}': must be lowercase letters, digits, and hyphens only"
        )));
    }

    let root = std::fs::canonicalize(path)
        .or_else(|_| {
            // Path may not exist yet; try to create it then canonicalize.
            std::fs::create_dir_all(path)?;
            std::fs::canonicalize(path)
        })
        .map_err(McpToolError::Io)?;

    let index_path = root.join("docs").join("arrows").join("index.yaml");
    if index_path.exists() {
        return Err(McpToolError::NotALidRepo(format!(
            "docs/arrows/index.yaml already exists at {}; project already initialized",
            root.display()
        )));
    }

    let arrows_seg_dir = root.join("docs").join("arrows").join(segment);
    let intent_dir = root.join("docs").join("intent");

    std::fs::create_dir_all(&arrows_seg_dir).map_err(McpToolError::Io)?;
    std::fs::create_dir_all(&intent_dir).map_err(McpToolError::Io)?;

    let index_yaml = format!(
        "schema_version: 2\narrows:\n  {segment}:\n    status: UNMAPPED\n    detail: {segment}/overview.md\n"
    );
    std::fs::write(&index_path, &index_yaml).map_err(McpToolError::Io)?;

    let overview_path = arrows_seg_dir.join("overview.md");
    let overview_md = format!(
        "# {segment}\n\n\
         ## Overview\n\n\
         <!-- Describe the {segment} segment here. -->\n\n\
         ## References\n\n\
         <!-- List related documents and spec files here. -->\n"
    );
    std::fs::write(&overview_path, &overview_md).map_err(McpToolError::Io)?;

    let rel = |p: &Path| {
        p.strip_prefix(&root)
            .unwrap_or(p)
            .to_string_lossy()
            .into_owned()
    };

    Ok(InitOutput {
        root: root.to_string_lossy().into_owned(),
        segment: segment.to_owned(),
        files_created: vec![rel(&index_path), rel(&overview_path), rel(&intent_dir)],
    })
}

fn is_valid_segment(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
