use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_router,
};

use crate::registry::RepoRegistry;
use crate::tools::{
    check::CheckInput,
    discover::DiscoverInput,
    find_refs::FindRefsInput,
    get_segment::GetSegmentInput,
    init::InitInput,
    list_segments::ListSegmentsInput,
    list_specs::ListSpecsInput,
    search::SearchInput,
    status::StatusInput,
    write_segment::{AddSegmentInput, UpdateSegmentInput},
    write_spec::{AddSpecInput, UpdateSpecStatusInput},
};

#[derive(Clone)]
pub struct LidMcpServer {
    registry: Arc<RepoRegistry>,
    #[allow(dead_code)] // required by #[tool_router] macro; accessed via generated code
    tool_router: ToolRouter<Self>,
}

impl Default for LidMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LidMcpServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RepoRegistry::new()),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl LidMcpServer {
    #[tool(
        description = "Scaffold a new LID project at `path`. Creates docs/arrows/index.yaml (schema v2), a stub arrow document, and docs/intent/. Accepts an optional `segment` name (default: \"core\"). Fails if a project already exists there. Returns the canonical root and list of created files."
    )]
    async fn lid_init(&self, Parameters(input): Parameters<InitInput>) -> String {
        crate::tools::init::lid_init(input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Discover a LID project at or above `path`. Registers it in the server and returns the canonical root, schema version, and segment count. Call this first before using any other tool."
    )]
    async fn lid_discover(&self, Parameters(input): Parameters<DiscoverInput>) -> String {
        crate::tools::discover::lid_discover(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Return an overview of a LID project: segments (with statuses) and spec counts (open / implemented / deferred)."
    )]
    async fn lid_status(&self, Parameters(input): Parameters<StatusInput>) -> String {
        crate::tools::status::lid_status(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Run coherence checks on a LID project and return all findings. Optionally filter to specific check IDs."
    )]
    async fn lid_check(&self, Parameters(input): Parameters<CheckInput>) -> String {
        crate::tools::check::lid_check(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "List all segments in the project's arrow index, including status, blocks, and detail path."
    )]
    async fn lid_list_segments(&self, Parameters(input): Parameters<ListSegmentsInput>) -> String {
        crate::tools::list_segments::lid_list_segments(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Get full details for one segment, including all its associated spec lines."
    )]
    async fn lid_get_segment(&self, Parameters(input): Parameters<GetSegmentInput>) -> String {
        crate::tools::get_segment::lid_get_segment(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "List all spec lines in the project. Optionally filter by segment ID prefix."
    )]
    async fn lid_list_specs(&self, Parameters(input): Parameters<ListSpecsInput>) -> String {
        crate::tools::list_specs::lid_list_specs(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(description = "Find all source-code locations that cite a spec ID via `@spec SPEC-ID`.")]
    async fn lid_find_spec_references(
        &self,
        Parameters(input): Parameters<FindRefsInput>,
    ) -> String {
        crate::tools::find_refs::lid_find_spec_references(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Case-insensitive substring search across segment IDs, spec IDs, and spec text."
    )]
    async fn lid_search(&self, Parameters(input): Parameters<SearchInput>) -> String {
        crate::tools::search::lid_search(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Add a new segment to the project's arrow index. Fails if the segment already exists."
    )]
    async fn lid_add_segment(&self, Parameters(input): Parameters<AddSegmentInput>) -> String {
        crate::tools::write_segment::lid_add_segment(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Update fields on an existing segment (status, next, drift). Omit fields that should not change."
    )]
    async fn lid_update_segment(
        &self,
        Parameters(input): Parameters<UpdateSegmentInput>,
    ) -> String {
        crate::tools::write_segment::lid_update_segment(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Update the status marker on a spec line (open / implemented / deferred). Atomically rewrites the spec file."
    )]
    async fn lid_update_spec_status(
        &self,
        Parameters(input): Parameters<UpdateSpecStatusInput>,
    ) -> String {
        crate::tools::write_spec::lid_update_spec_status(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }

    #[tool(
        description = "Append a new open spec line to a segment's spec file. Creates the file (with a header) if it does not exist. Fails if the spec ID already exists."
    )]
    async fn lid_add_spec(&self, Parameters(input): Parameters<AddSpecInput>) -> String {
        crate::tools::write_spec::lid_add_spec(&self.registry, input)
            .await
            .unwrap_or_else(|e| format!("error: {}", e.message))
    }
}

impl ServerHandler for LidMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "LID MCP server — navigate and modify LID (Linked-Intent Development) projects. \
                 Call lid_discover first to register a project, then use the other tools. \
                 All tools accept `project_root` as the canonical root returned by lid_discover.",
        )
    }
}
