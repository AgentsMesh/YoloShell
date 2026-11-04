use super::*;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::{
    CodexAuthorityStatus, CodexAuthorityVersion, CodexBindingAuthority,
};
use crate::agent::hooks::{Generation, ProcessIdentity};
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use uuid::Uuid;

const ROUTE_REVISION: u64 = 7;
const VERSION_INCREMENT: u64 = 1;
const PROVIDER_PID: u32 = 41;
const PROVIDER_START_TIME: u64 = 42;
const WORKER_PID: u32 = 51;
const WORKER_START_TIME: u64 = 52;
const AGENT_SESSION_ID: &str = "0198f5ec-ec51-7f88-b0af-76f5bf7c3c11";
const CWD: &str = "/tmp/codex-projection-revocation";
const EVIDENCE_PATH: &str = "/tmp/codex-projection-revocation/rollout.jsonl";
const DATA_HOME: &str = "/tmp/codex-projection-revocation/codex-home";

#[test]
fn later_events_cannot_resurrect_a_revoked_route_but_a_new_route_can() {
    let active = authority(ROUTE_REVISION);
    let revoked = active.revoked_for_test().unwrap();
    let mut later_generation = active.clone();
    later_generation.version.generation =
        Generation(active.version.generation.0 + VERSION_INCREMENT);
    let mut later_revision = active.clone();
    later_revision.version.authority_revision += VERSION_INCREMENT;
    let mut later_sequence = later_revision.clone();
    later_sequence.version.sequence += VERSION_INCREMENT;

    for event in [&active, &later_generation, &later_revision, &later_sequence] {
        assert!(publication_is_blocked(Some(&revoked), event));
    }

    let next_route = authority(ROUTE_REVISION + VERSION_INCREMENT);
    assert!(!publication_is_blocked(Some(&revoked), &next_route));
}

fn authority(route_revision: u64) -> CodexBindingAuthority {
    CodexBindingAuthority::new(
        AuthorityRuntimeIdentity {
            runtime_epoch: Uuid::new_v4(),
            worker: AuthorityProcessIdentity {
                pid: WORKER_PID,
                start_time: WORKER_START_TIME,
            },
        },
        ProcessIdentity::new(PROVIDER_PID, PROVIDER_START_TIME),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        CodexAuthorityVersion {
            route_revision,
            invocation_id: Uuid::new_v4(),
            generation: Generation::INITIAL,
            authority_revision: VERSION_INCREMENT,
            sequence: VERSION_INCREMENT,
        },
        CodexAuthorityStatus::Active,
        binding(),
    )
    .unwrap()
}

fn binding() -> AgentBinding {
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: AgentKind::Codex,
        provider_session_id: AGENT_SESSION_ID.to_owned(),
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        cwd: CWD.to_owned(),
        evidence_path: EVIDENCE_PATH.to_owned(),
        data_home: DATA_HOME.to_owned(),
        process_pid: Some(PROVIDER_PID),
        process_start_time: Some(PROVIDER_START_TIME),
        state: AgentState::Running,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: VERSION_INCREMENT as i64,
        updated_at_ms: (VERSION_INCREMENT + VERSION_INCREMENT) as i64,
    }
}
