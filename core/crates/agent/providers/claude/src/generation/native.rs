mod binding;
mod evidence;
mod reducer;
mod replay_validation;
mod revocation;
mod session_origin;
mod store;
mod validation;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_replay_identity_tests.rs"]
mod replay_identity_tests;

use crate::agent::{AgentBinding, AgentState};
use crate::claude_hooks::{ClaudeNativeRequest, ClaudeNativeScope};
use crate::paths::identity::ObjectIdentity;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

pub use store::ClaudeNativeGenerationStore;

pub const NATIVE_GENERATION_FILE: &str = "claude-native-generation.json";
const SCHEMA_VERSION: u32 = 1;
const INITIAL_REVISION: u64 = 1;
const INCREMENT: u64 = 1;
const INVALID: &str = "Claude native generation authority is invalid or changed";
const REVOKED: &str = "Claude native invocation was explicitly revoked";
const OUT_OF_ORDER: &str = "Claude native event is conflicting or out of order";
const CAPACITY: &str = "Claude native invocation capacity is exhausted";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeNativeGeneration {
    pub schema_version: u32,
    pub scope: ClaudeNativeScope,
    pub worker: super::AuthorityProcessIdentity,
    pub boot_id: String,
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: Uuid,
    pub revision: u64,
    pub last_event_sequence: u64,
    pub activity: AgentState,
    pub final_session_end: bool,
    pub revoked: bool,
    pub binding: AgentBinding,
    pub transcript_identity: ObjectIdentity,
    pub cwd_identity: ObjectIdentity,
    pub data_home_identity: ObjectIdentity,
    pub replay_events: VecDeque<ClaudeNativeRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeGenerations {
    schema_version: u32,
    #[serde(with = "crate::serde_uuid")]
    active_invocation: Uuid,
    invocations: Vec<ClaudeNativeGeneration>,
}

impl ClaudeNativeGeneration {
    pub fn verify_resume_evidence(&self) -> anyhow::Result<bool> {
        evidence::verify(self)
    }

    pub fn matches_runtime(&self, runtime: super::AuthorityRuntimeIdentity) -> bool {
        self.scope.runtime_epoch == runtime.runtime_epoch && self.worker == runtime.worker
    }

    pub fn process(&self) -> crate::process::ProcessIdentity {
        crate::process::ProcessIdentity {
            pid: self.scope.agent.pid,
            start_time: self.scope.agent.start_time,
        }
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_record_corruption_tests.rs"]
mod corruption_tests;
#[cfg(test)]
#[path = "../../test_support/native_generation.rs"]
mod fixture;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_generation_tests.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_resume_origin_tests.rs"]
mod resume_origin_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native_generation_tests.rs"]
mod tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native_generation_validation_tests.rs"]
mod validation_tests;
