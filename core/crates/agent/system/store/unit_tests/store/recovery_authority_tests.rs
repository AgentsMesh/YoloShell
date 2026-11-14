use super::recovery_authority::BindingRecoveryAuthority;
use super::*;
use crate::agent::constants::BINDING_SCHEMA_VERSION;
use crate::agent::{AgentKind, AgentRecoveryAvailability};
use uuid::Uuid;

const AGENT_SESSION_ID: &str = "provider-session";
const FORGED_AGENT_SESSION_ID: &str = "forged-provider-session";
const CWD: &str = "/tmp/provider-project";
const EVIDENCE_PATH: &str = "/tmp/provider-home/sessions/evidence.jsonl";
const DATA_HOME: &str = "/tmp/provider-home";
const PROCESS_PID: u32 = 901;
const PROCESS_START_TIME: u64 = 902;
const DETECTED_AT_MS: i64 = 100;
const UPDATED_AT_MS: i64 = 200;

struct ExactAuthority(AgentBinding);

impl BindingRecoveryAuthority for ExactAuthority {
    fn authorizes(&self, binding: &AgentBinding) -> bool {
        self.0 == *binding
    }
}

#[test]
fn provider_authority_preserves_only_its_exact_active_binding() {
    let canonical = binding(AgentState::Active, AgentRecoveryAvailability::Available);
    let authority = ExactAuthority(canonical.clone());
    assert_eq!(
        recovery_safe(
            canonical.clone(),
            &yolo_agent_providers::builtin_agent_registry(),
            Some(&authority),
        ),
        canonical
    );

    let mut forged = authority.0.clone();
    forged.provider_session_id = FORGED_AGENT_SESSION_ID.to_owned();
    assert_failed_closed(recovery_safe(forged, &yolo_agent_providers::builtin_agent_registry(), Some(&authority)), AgentState::Unknown);
}

#[test]
fn provider_authority_preserves_its_exact_confirmed_exit() {
    let canonical = binding(AgentState::Exited, AgentRecoveryAvailability::Available);
    let authority = ExactAuthority(canonical.clone());
    assert_eq!(
        recovery_safe(
            canonical.clone(),
            &yolo_agent_providers::builtin_agent_registry(),
            Some(&authority),
        ),
        canonical
    );
}

#[test]
fn authority_cannot_bypass_the_hook_required_state_matrix() {
    let forged = binding(AgentState::Unknown, AgentRecoveryAvailability::Available);
    let authority = ExactAuthority(forged.clone());
    assert_failed_closed(recovery_safe(forged, &yolo_agent_providers::builtin_agent_registry(), Some(&authority)), AgentState::Unknown);

    for state in [
        AgentState::Active,
        AgentState::Running,
        AgentState::WaitingForInput,
    ] {
        for availability in [
            AgentRecoveryAvailability::UnmanagedLaunch,
            AgentRecoveryAvailability::Unavailable,
        ] {
            let forged = binding(state, availability);
            let authority = ExactAuthority(forged.clone());
            assert_failed_closed(recovery_safe(forged, &yolo_agent_providers::builtin_agent_registry(), Some(&authority)), AgentState::Unknown);
        }
    }
    let forged = binding(
        AgentState::Exited,
        AgentRecoveryAvailability::UnmanagedLaunch,
    );
    let authority = ExactAuthority(forged.clone());
    assert_failed_closed(recovery_safe(forged, &yolo_agent_providers::builtin_agent_registry(), Some(&authority)), AgentState::Exited);
}

#[test]
fn absent_authority_preserves_only_safe_unmanaged_or_exited_shapes() {
    let unmanaged = binding(
        AgentState::Unknown,
        AgentRecoveryAvailability::UnmanagedLaunch,
    );
    assert_eq!(recovery_safe(unmanaged.clone(), &yolo_agent_providers::builtin_agent_registry(), None), unmanaged);
    for availability in [
        AgentRecoveryAvailability::Available,
        AgentRecoveryAvailability::Unavailable,
    ] {
        assert_failed_closed(
            recovery_safe(
                binding(AgentState::Unknown, availability),
                &yolo_agent_providers::builtin_agent_registry(),
                None,
            ),
            AgentState::Unknown,
        );
    }

    for state in [
        AgentState::Active,
        AgentState::Running,
        AgentState::WaitingForInput,
    ] {
        for availability in [
            AgentRecoveryAvailability::Available,
            AgentRecoveryAvailability::UnmanagedLaunch,
            AgentRecoveryAvailability::Unavailable,
        ] {
            assert_failed_closed(
                recovery_safe(binding(state, availability), &yolo_agent_providers::builtin_agent_registry(), None),
                AgentState::Unknown,
            );
        }
    }
    for availability in [
        AgentRecoveryAvailability::Available,
        AgentRecoveryAvailability::UnmanagedLaunch,
        AgentRecoveryAvailability::Unavailable,
    ] {
        assert_failed_closed(
            recovery_safe(
                binding(AgentState::Exited, availability),
                &yolo_agent_providers::builtin_agent_registry(),
                None,
            ),
            AgentState::Exited,
        );
    }
}

fn assert_failed_closed(binding: AgentBinding, state: AgentState) {
    assert_eq!(binding.state, state);
    assert_eq!(
        binding.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
}

fn binding(state: AgentState, recovery_availability: AgentRecoveryAvailability) -> AgentBinding {
    let active = state != AgentState::Exited;
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: AgentKind::Codex,
        provider_session_id: AGENT_SESSION_ID.to_owned(),
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        cwd: CWD.to_owned(),
        evidence_path: EVIDENCE_PATH.to_owned(),
        data_home: DATA_HOME.to_owned(),
        process_pid: active.then_some(PROCESS_PID),
        process_start_time: active.then_some(PROCESS_START_TIME),
        state,
        recovery_availability,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    }
}
