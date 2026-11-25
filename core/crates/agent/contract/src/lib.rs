#[path = "constants.rs"]
mod constants;
#[path = "kind.rs"]
mod kind;
#[path = "types.rs"]
mod types;

pub use constants::{
    BINDING_FILE_NAME, CLAUDE_CODE_KIND, CODEX_KIND, PROVIDER_CAPABILITY_PREFIX,
    PROVIDER_CATALOG_CAPABILITY, ZERO_PROCESS_GROUP, ZERO_PROCESS_START_TIME,
};
pub use kind::AgentKind;
pub use types::{
    AgentBinding, AgentEvidence, AgentRecoveryAvailability, AgentState, DetectionResult,
    ObservedProcess, ResumePlan,
};
