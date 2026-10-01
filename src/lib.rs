#![forbid(unsafe_code)]
#![doc = "Provider-neutral version-control observations and bounded local adapters."]

mod git;

pub use git::GitObserver;
use serde::{Deserialize, Serialize};

pub const SOURCE_REVISION_OBSERVATION_SCHEMA_V1: &str =
    "zixcel://version-control/source-revision-observation/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkingState {
    Clean,
    Modified,
    Untracked,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRevisionObservation {
    pub schema: String,
    pub observation_ref: String,
    pub provider: String,
    pub repository_ref: String,
    pub immutable_revision_ref: Option<String>,
    pub parent_revision_refs: Vec<String>,
    pub content_root_ref: Option<String>,
    pub branch_hint: Option<String>,
    pub working_state: WorkingState,
    pub changed_entry_count: u32,
    pub observed_at: String,
    pub evidence_digest_sha256: String,
    pub adapter_ref: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ObservationError {
    #[error("version-control root is unavailable: {0}")]
    Unavailable(String),
    #[error("version-control adapter could not be started")]
    AdapterUnavailable,
    #[error("version-control observation exceeded its bounded output")]
    OutputLimit,
    #[error("version-control repository returned invalid data: {0}")]
    Invalid(String),
}
