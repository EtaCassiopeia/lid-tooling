use rmcp::model::ErrorData;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpToolError {
    #[error("not a LID repository: {0}")]
    NotALidRepo(String),

    #[error("schema_version {found} is not supported (supported: {supported:?})")]
    UnsupportedSchemaVersion {
        found: u32,
        supported: &'static [u32],
    },

    #[error("segment not found: {0}")]
    SegmentNotFound(String),

    #[error("spec not found: {0}")]
    SpecNotFound(String),

    #[error("segment already exists: {0}")]
    DuplicateSegment(String),

    #[error("spec already exists: {0}")]
    DuplicateSpec(String),

    #[error("invalid segment id: {0}")]
    InvalidSegmentId(String),

    #[error("invalid spec id: {0}")]
    InvalidSpecId(String),

    #[error("invalid status: {0}")]
    InvalidStatus(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(String),
}

impl From<McpToolError> for ErrorData {
    fn from(e: McpToolError) -> Self {
        ErrorData::invalid_params(e.to_string(), None)
    }
}
