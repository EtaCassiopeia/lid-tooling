use std::path::{Path, PathBuf};

use lid_core::model::{SpecId, SpecStatus};
use lid_core::parse::markdown;
use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::McpToolError;
use crate::registry::RepoRegistry;

// ── lid_update_spec_status ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateSpecStatusInput {
    pub project_root: String,
    /// The spec ID to update (e.g. "AUTH-UI-001").
    pub spec_id: String,
    /// New status: "open", "implemented", or "deferred".
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct WriteResult {
    pub action: String,
    pub rediscover_error: Option<String>,
}

pub async fn lid_update_spec_status(
    registry: &RepoRegistry,
    input: UpdateSpecStatusInput,
) -> Result<String, ErrorData> {
    let new_status = parse_status(&input.status).map_err(ErrorData::from)?;
    let spec_id = SpecId::parse(&input.spec_id)
        .map_err(|_| McpToolError::InvalidSpecId(input.spec_id.clone()))
        .map_err(ErrorData::from)?;

    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let spec_path = {
        let repo = handle.read().await;
        repo.specs
            .iter()
            .find(|sf| sf.specs.iter().any(|sl| sl.id == spec_id))
            .map(|sf| repo.root.join(&sf.path))
            .ok_or_else(|| McpToolError::SpecNotFound(input.spec_id.clone()))
            .map_err(ErrorData::from)?
    };

    rewrite_spec_status(&spec_path, &spec_id, new_status)
        .await
        .map_err(ErrorData::from)?;

    let rediscover_error = registry.rediscover(Path::new(&input.project_root)).await;

    let out = WriteResult {
        action: format!("updated {} to {}", input.spec_id, new_status),
        rediscover_error,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

// ── lid_update_spec_text ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateSpecTextInput {
    pub project_root: String,
    /// The spec ID whose text to update (e.g. "AUTH-001").
    pub spec_id: String,
    /// The new requirement text (without the ID prefix).
    pub new_text: String,
}

pub async fn lid_update_spec_text(
    registry: &RepoRegistry,
    input: UpdateSpecTextInput,
) -> Result<String, ErrorData> {
    let spec_id = SpecId::parse(&input.spec_id)
        .map_err(|_| McpToolError::InvalidSpecId(input.spec_id.clone()))
        .map_err(ErrorData::from)?;

    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let spec_path = {
        let repo = handle.read().await;
        repo.specs
            .iter()
            .find(|sf| sf.specs.iter().any(|sl| sl.id == spec_id))
            .map(|sf| repo.root.join(&sf.path))
            .ok_or_else(|| McpToolError::SpecNotFound(input.spec_id.clone()))
            .map_err(ErrorData::from)?
    };

    let content = tokio::fs::read_to_string(&spec_path)
        .await
        .map_err(McpToolError::Io)
        .map_err(ErrorData::from)?;
    let updated = markdown::update_spec_text_in_text(&content, &spec_id, &input.new_text)
        .ok_or_else(|| McpToolError::SpecNotFound(input.spec_id.clone()))
        .map_err(ErrorData::from)?;
    atomic_write(&spec_path, &updated)
        .await
        .map_err(ErrorData::from)?;

    let rediscover_error = registry.rediscover(Path::new(&input.project_root)).await;

    let out = WriteResult {
        action: format!("updated text of {}", input.spec_id),
        rediscover_error,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

// ── lid_add_spec ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddSpecInput {
    pub project_root: String,
    /// The segment this spec belongs to (e.g. "auth").
    pub segment_id: String,
    /// The new spec ID (e.g. "AUTH-UI-042").
    pub spec_id: String,
    /// The requirement text (without the ID prefix).
    pub text: String,
}

pub async fn lid_add_spec(
    registry: &RepoRegistry,
    input: AddSpecInput,
) -> Result<String, ErrorData> {
    let spec_id = SpecId::parse(&input.spec_id)
        .map_err(|_| McpToolError::InvalidSpecId(input.spec_id.clone()))
        .map_err(ErrorData::from)?;

    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    {
        let repo = handle.read().await;
        // Reject duplicate spec IDs.
        if repo
            .specs
            .iter()
            .flat_map(|sf| sf.specs.iter())
            .any(|sl| sl.id == spec_id)
        {
            return Err(ErrorData::from(McpToolError::DuplicateSpec(
                input.spec_id.clone(),
            )));
        }
    }

    let seg = input.segment_id.to_lowercase();
    let specs_path = Path::new(&input.project_root)
        .join("docs")
        .join("intent")
        .join(&seg)
        .join(format!("{seg}-specs.md"));

    // Validate prefix when the spec file already exists and declares one.
    let declared_prefix = {
        let repo = handle.read().await;
        let specs_rel = PathBuf::from("docs")
            .join("intent")
            .join(&seg)
            .join(format!("{seg}-specs.md"));
        repo.specs
            .iter()
            .find(|sf| sf.path == specs_rel)
            .and_then(|sf| sf.prefix.clone())
    };
    if let Some(ref prefix) = declared_prefix {
        let expected = format!("{prefix}-");
        if !input.spec_id.starts_with(&expected) {
            return Err(ErrorData::from(McpToolError::InvalidSpecId(format!(
                "spec ID {} does not start with declared prefix {}",
                input.spec_id, prefix
            ))));
        }
    }

    let derived_prefix = declared_prefix.or_else(|| derive_prefix_from_id(&input.spec_id));
    append_spec_line(
        &specs_path,
        &input.spec_id,
        &input.text,
        derived_prefix.as_deref(),
    )
    .await
    .map_err(ErrorData::from)?;

    let rediscover_error = registry.rediscover(Path::new(&input.project_root)).await;

    let out = WriteResult {
        action: format!("added spec {} to {}", input.spec_id, specs_path.display()),
        rediscover_error,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_status(s: &str) -> Result<SpecStatus, McpToolError> {
    match s.to_lowercase().as_str() {
        "open" => Ok(SpecStatus::Open),
        "implemented" => Ok(SpecStatus::Implemented),
        "deferred" => Ok(SpecStatus::Deferred),
        other => Err(McpToolError::InvalidStatus(other.to_owned())),
    }
}

async fn rewrite_spec_status(
    path: &Path,
    spec_id: &SpecId,
    new_status: SpecStatus,
) -> Result<(), McpToolError> {
    let content = tokio::fs::read_to_string(path).await?;
    let updated = markdown::update_spec_status_in_text(&content, spec_id, new_status)
        .ok_or_else(|| McpToolError::SpecNotFound(spec_id.to_string()))?;
    atomic_write(path, &updated).await
}

async fn append_spec_line(
    path: &Path,
    spec_id: &str,
    text: &str,
    prefix: Option<&str>,
) -> Result<(), McpToolError> {
    let line = format!("- [ ] **{spec_id}**: {text}\n");
    if path.exists() {
        let mut existing = tokio::fs::read_to_string(path).await?;
        if !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str(&line);
        atomic_write(path, &existing).await
    } else {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let seg = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("segment")
            .trim_end_matches("-specs");
        let frontmatter = prefix.map_or(String::new(), |p| format!("---\nprefix: {p}\n---\n\n"));
        let header = format!("{frontmatter}# {seg} specs\n\n");
        atomic_write(path, &(header + &line)).await
    }
}

fn derive_prefix_from_id(spec_id: &str) -> Option<String> {
    let last = spec_id.rfind('-')?;
    let last_seg = &spec_id[last + 1..];
    if !last_seg.is_empty() && last_seg.chars().all(|c| c.is_ascii_digit()) {
        Some(spec_id[..last].to_owned())
    } else {
        None
    }
}

async fn atomic_write(path: &Path, content: &str) -> Result<(), McpToolError> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, content).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}
