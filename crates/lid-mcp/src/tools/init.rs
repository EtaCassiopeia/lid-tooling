use std::path::Path;

use lid_core::scaffold;
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
    /// Spec-ID prefix for the first segment (e.g. "MYAPP"). Defaults to the
    /// uppercased segment name.
    #[serde(default)]
    pub spec_prefix: Option<String>,
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
    let spec_prefix = input
        .spec_prefix
        .clone()
        .unwrap_or_else(|| segment.to_uppercase());
    tokio::task::spawn_blocking(move || init_blocking(&path, &segment, &spec_prefix))
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(ErrorData::from)
        .and_then(|out| {
            serde_json::to_string_pretty(&out)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))
        })
}

fn init_blocking(path: &str, segment: &str, spec_prefix: &str) -> Result<InitOutput, McpToolError> {
    if !is_valid_segment(segment) {
        return Err(McpToolError::InvalidSegmentId(format!(
            "'{segment}': must be lowercase letters, digits, and hyphens only"
        )));
    }

    let root = std::fs::canonicalize(path)
        .or_else(|_| {
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

    std::fs::create_dir_all(root.join("docs").join("arrows").join(segment))
        .map_err(McpToolError::Io)?;

    let detail = format!("{segment}/overview.md");
    let index_yaml = format!(
        "schema_version: 2\narrows:\n  {segment}:\n    status: UNMAPPED\n    detail: {detail}\n"
    );
    std::fs::write(&index_path, &index_yaml).map_err(McpToolError::Io)?;

    let arrow_path = scaffold::scaffold_arrow_doc(&root, &detail, segment, None)
        .map_err(McpToolError::Io)?
        .ok_or_else(|| McpToolError::Io(std::io::Error::other("arrow doc already exists")))?;

    let intent_files = scaffold::scaffold_intent_dir(&root, None, segment, spec_prefix)
        .map_err(McpToolError::Io)?;

    let rel = |p: &Path| {
        p.strip_prefix(&root)
            .unwrap_or(p)
            .to_string_lossy()
            .into_owned()
    };

    let mut files_created = vec![rel(&index_path), rel(&arrow_path)];
    files_created.extend(intent_files.iter().map(|p| rel(p)));

    Ok(InitOutput {
        root: root.to_string_lossy().into_owned(),
        segment: segment.to_owned(),
        files_created,
    })
}

fn is_valid_segment(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
