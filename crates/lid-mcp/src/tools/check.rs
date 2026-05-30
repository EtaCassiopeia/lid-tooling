use lid_core::checks;
use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckInput {
    /// Absolute path to the LID project root.
    pub project_root: String,
    /// Optional list of check IDs to run. When absent all checks run.
    #[serde(default)]
    pub checks: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FindingSummary {
    check: String,
    severity: String,
    category: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remediation: Option<String>,
}

#[derive(Debug, Serialize)]
struct CheckOutput {
    root: String,
    total: usize,
    errors: usize,
    warnings: usize,
    findings: Vec<FindingSummary>,
}

pub async fn lid_check(registry: &RepoRegistry, input: CheckInput) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;
    let repo_clone = repo.clone();
    drop(repo);

    let filter: Vec<String> = input.checks;
    let findings = tokio::task::spawn_blocking(move || {
        let mut all = Vec::new();
        for check in checks::default_checks() {
            if !filter.is_empty() && !filter.contains(&check.id().to_string()) {
                continue;
            }
            all.extend(check.run(&repo_clone));
        }
        all
    })
    .await
    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

    let errors = findings
        .iter()
        .filter(|f| f.severity == lid_core::Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity == lid_core::Severity::Warning)
        .count();

    let summaries = findings
        .into_iter()
        .map(|f| FindingSummary {
            check: f.check.to_string(),
            severity: f.severity.to_string(),
            category: f.category.to_string(),
            message: f.message,
            location: f.location.map(|l| l.path.to_string_lossy().into_owned()),
            spec: f.spec.map(|s| s.to_string()),
            remediation: f.remediation,
        })
        .collect();

    let out = CheckOutput {
        root: input.project_root,
        total: errors + warnings,
        errors,
        warnings,
        findings: summaries,
    };
    serde_json::to_string_pretty(&out).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
