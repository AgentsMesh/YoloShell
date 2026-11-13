use super::super::{GenerationEvent, SessionStartSource};
use super::support::*;
use crate::agent::{AgentBinding, AgentBindingStore, AgentRecoveryAvailability, AgentState};

const ABSOLUTE_CLAUDE: &str = "/managed/agent-bin/claude";

#[test]
fn detected_running_downgrades_stale_waiting_hook_state_but_retains_resume_identity() {
    let fixture = Fixture::new();
    let session_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let mut start = fixture.start(&session_id, SessionStartSource::Fresh);
    let GenerationEvent::SessionStart { state, .. } = &mut start else {
        unreachable!();
    };
    *state = AgentState::WaitingForInput;
    let authority = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &start)
        .unwrap()
        .0;
    let binding = binding(&authority);
    let store = AgentBindingStore::new(fixture.binding_path());
    store
        .save_authoritative_in_namespace(&fixture.lease, &binding, &authority, fixture.runtime())
        .unwrap();
    let mut detected = binding.clone();
    detected.state = AgentState::Running;

    let downgraded = store
        .reconcile_detected_in_namespace(&fixture.lease, &detected, fixture.runtime())
        .unwrap();

    assert_eq!(downgraded.state, AgentState::Unknown);
    assert_eq!(downgraded.provider_session_id, binding.provider_session_id);
    assert_eq!(downgraded.evidence_path, binding.evidence_path);
    assert_eq!(downgraded.process_pid, binding.process_pid);
}

#[test]
fn canonical_binding_survives_hook_required_rescan_and_monitor_restart() {
    for state in [AgentState::Running, AgentState::WaitingForInput] {
        assert_canonical_binding_survives_rescan_and_restart(state);
    }
}

fn assert_canonical_binding_survives_rescan_and_restart(state: AgentState) {
    let (fixture, authority, store, canonical) = published_with_state(state);
    let detected = unmanaged_scan(&canonical);

    let retained = store
        .reconcile_detected_in_namespace(&fixture.lease, &detected, fixture.runtime())
        .unwrap();
    assert_eq!(retained, canonical);

    let mut legacy = canonical.clone();
    legacy.state = AgentState::Unknown;
    fixture
        .lease
        .write_json(&fixture.binding_path(), &legacy)
        .unwrap();
    let restored = store
        .reconcile_detected_in_namespace(&fixture.lease, &detected, fixture.runtime())
        .unwrap();
    assert!(authority.matches_binding(&restored));
    assert_eq!(
        restored.recovery_availability,
        AgentRecoveryAvailability::Available
    );
}

#[test]
fn rejected_unmanaged_state_scans_preserve_canonical_binding() {
    for state in [AgentState::Running, AgentState::WaitingForInput] {
        let (fixture, authority, store, canonical) = published();
        let mut detected = unmanaged_scan(&canonical);
        detected.state = state;
        let reconciled = store
            .reconcile_detected_in_namespace(&fixture.lease, &detected, fixture.runtime())
            .unwrap();
        assert!(authority.matches_binding(&reconciled));
        assert_eq!(reconciled, canonical);
    }
}

#[test]
fn non_scanner_availability_and_stale_runtime_cannot_restore_canonical_binding() {
    for availability in [
        AgentRecoveryAvailability::Available,
        AgentRecoveryAvailability::Unavailable,
    ] {
        let (fixture, authority, store, canonical) = published();
        let mut detected = unmanaged_scan(&canonical);
        detected.recovery_availability = availability;
        let reconciled = store
            .reconcile_detected_in_namespace(&fixture.lease, &detected, fixture.runtime())
            .unwrap();
        assert!(!authority.matches_binding(&reconciled));
    }
    let (fixture, authority, store, canonical) = published();
    let mut stale_runtime = fixture.runtime();
    stale_runtime.runtime_epoch = uuid::Uuid::new_v4();
    let reconciled = store
        .reconcile_detected_in_namespace(&fixture.lease, &unmanaged_scan(&canonical), stale_runtime)
        .unwrap();
    assert!(!authority.matches_binding(&reconciled));
}

fn published() -> (
    Fixture,
    super::super::ClaudeGenerationAuthority,
    AgentBindingStore,
    AgentBinding,
) {
    published_with_state(AgentState::Running)
}

fn published_with_state(
    state: AgentState,
) -> (
    Fixture,
    super::super::ClaudeGenerationAuthority,
    AgentBindingStore,
    AgentBinding,
) {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let mut start = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
    let GenerationEvent::SessionStart {
        state: start_state, ..
    } = &mut start
    else {
        unreachable!();
    };
    *start_state = state;
    let authority = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &start)
        .unwrap()
        .0;
    let canonical = binding(&authority);
    let store = AgentBindingStore::new(fixture.binding_path());
    store
        .save_authoritative_in_namespace(&fixture.lease, &canonical, &authority, fixture.runtime())
        .unwrap();
    (fixture, authority, store, canonical)
}

fn unmanaged_scan(canonical: &AgentBinding) -> AgentBinding {
    let mut detected = canonical.clone();
    detected.executable = ABSOLUTE_CLAUDE.to_owned();
    detected.state = AgentState::Unknown;
    detected.recovery_availability = AgentRecoveryAvailability::UnmanagedLaunch;
    detected
}
