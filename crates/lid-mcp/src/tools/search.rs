use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::RepoRegistry;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchInput {
    pub project_root: String,
    /// Case-insensitive substring to match against segment IDs, spec IDs, and spec text.
    pub query: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
enum SearchHit {
    Segment {
        id: String,
        status: String,
    },
    Spec {
        id: String,
        status: String,
        text: String,
        file: String,
    },
}

pub async fn lid_search(registry: &RepoRegistry, input: SearchInput) -> Result<String, ErrorData> {
    let handle = registry
        .get_or_discover(&input.project_root)
        .await
        .map_err(ErrorData::from)?;

    let repo = handle.read().await;
    let q = input.query.to_lowercase();

    let mut hits: Vec<SearchHit> = Vec::new();

    for (id, seg) in &repo.index.arrows {
        if id.as_ref().to_lowercase().contains(&q) {
            hits.push(SearchHit::Segment {
                id: id.to_string(),
                status: format!("{:?}", seg.status),
            });
        }
    }

    for sf in &repo.specs {
        for sl in &sf.specs {
            if sl.id.as_ref().to_lowercase().contains(&q) || sl.text.to_lowercase().contains(&q) {
                hits.push(SearchHit::Spec {
                    id: sl.id.to_string(),
                    status: format!("{:?}", sl.status),
                    text: sl.text.clone(),
                    file: sf.path.to_string_lossy().into_owned(),
                });
            }
        }
    }

    serde_json::to_string_pretty(&hits).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}
