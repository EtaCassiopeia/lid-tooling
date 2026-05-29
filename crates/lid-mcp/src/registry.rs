use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lid_core::{LidError, LidRepo};
use tokio::sync::RwLock;

use crate::error::McpToolError;

pub struct RepoRegistry {
    map: RwLock<HashMap<PathBuf, Arc<RwLock<LidRepo>>>>,
}

impl Default for RepoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RepoRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Return the repo for `path`, discovering it on first access.
    ///
    /// `path` is canonicalized so `/foo/../bar` and `/bar` hit the same entry.
    pub async fn get_or_discover(&self, path: &str) -> Result<Arc<RwLock<LidRepo>>, McpToolError> {
        let canonical =
            std::fs::canonicalize(path).map_err(|_| McpToolError::NotALidRepo(path.to_owned()))?;

        {
            let map = self.map.read().await;
            if let Some(repo) = map.get(&canonical) {
                return Ok(Arc::clone(repo));
            }
        }

        let discover_path = canonical.clone();
        let repo = tokio::task::spawn_blocking(move || LidRepo::discover(&discover_path))
            .await
            .map_err(|e| McpToolError::NotALidRepo(e.to_string()))?
            .map_err(|e| match e {
                LidError::NotALidRepo { .. } => McpToolError::NotALidRepo(path.to_owned()),
                LidError::UnsupportedSchemaVersion { found, supported } => {
                    McpToolError::UnsupportedSchemaVersion { found, supported }
                }
                other => McpToolError::NotALidRepo(other.to_string()),
            })?;

        let handle = Arc::new(RwLock::new(repo));
        let mut map = self.map.write().await;
        let entry = map.entry(canonical).or_insert_with(|| Arc::clone(&handle));
        Ok(Arc::clone(entry))
    }

    /// Re-discover the repo rooted at `root`, replacing the stored snapshot.
    ///
    /// On failure, keeps the old snapshot and returns the error string so the
    /// caller can include it in the tool result without losing state.
    pub async fn rediscover(&self, root: &Path) -> Option<String> {
        let canonical = match std::fs::canonicalize(root) {
            Ok(p) => p,
            Err(e) => return Some(e.to_string()),
        };

        let discover_path = canonical.clone();
        let result = tokio::task::spawn_blocking(move || LidRepo::discover(&discover_path))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()));

        let map = self.map.read().await;
        if let Some(handle) = map.get(&canonical) {
            match result {
                Ok(fresh) => *handle.write().await = fresh,
                Err(e) => return Some(e),
            }
        }
        None
    }
}
