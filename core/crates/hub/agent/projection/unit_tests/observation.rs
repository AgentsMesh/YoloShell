use super::{
    fixture::record,
    view::{exit, view},
};
use crate::observation::normalize;
use uuid::Uuid;
use yolo_agent_contract::{AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_agent_provider_contract::ProjectionPolicy;
use yolo_hub_agent_protocol::*;
use yolo_terminal_model::{RecoveryState, TerminalRuntimeStatus, TerminalView};

fn fields_of(source: &TerminalView) -> AgentSnapshotFields {
    record(source)
        .entries
        .values()
        .next()
        .unwrap()
        .snapshot
        .fields()
        .clone()
}

fn normalized(
    source: &TerminalView,
    prompt_ready: bool,
    policy: ProjectionPolicy,
) -> AgentSnapshotFields {
    let mut fields = fields_of(source);
    normalize(
        &mut fields,
        source.agent.as_ref().unwrap(),
        source,
        prompt_ready,
        policy,
    );
    fields
}

#[test]
fn authenticated_hook_states_map_without_inventing_semantic_control() {
    for (state, activity) in [
        (AgentState::Active, Activity::Unknown),
        (AgentState::Running, Activity::Working),
        (AgentState::WaitingForInput, Activity::WaitingForInput),
        (
            AgentState::WaitingForPermission,
            Activity::WaitingForApproval,
        ),
        (AgentState::Unknown, Activity::Unknown),
    ] {
        let mut source = view();
        source.agent.as_mut().unwrap().state = state;
        let fields = normalized(&source, false, ProjectionPolicy::HOOK_AUTHORITATIVE);
        assert_eq!(fields.activity, activity);
        assert_eq!(fields.source, ObservationSource::ProviderHook);
        assert_eq!(fields.capabilities, AgentCapabilities::UNSUPPORTED);
        assert_eq!(fields.interaction, None);
        assert_eq!(fields.recovery, RecoveryStatus::Available);
        assert_eq!(
            fields.lifecycle,
            if state == AgentState::Unknown {
                Lifecycle::Unknown {}
            } else {
                Lifecycle::Running {}
            }
        );
        assert_eq!(
            fields.freshness,
            if state == AgentState::Unknown {
                Freshness::Stale
            } else {
                Freshness::Current
            }
        );
        assert!(AgentSnapshot::new(fields).is_ok());
    }
}

#[test]
fn unavailable_runtime_is_not_proof_of_agent_exit_or_fresh_hook_state() {
    for status in [
        TerminalRuntimeStatus::Starting,
        TerminalRuntimeStatus::Running,
        TerminalRuntimeStatus::Finalizing,
        TerminalRuntimeStatus::OutputBlocked,
        TerminalRuntimeStatus::Exited,
        TerminalRuntimeStatus::Unreachable,
    ] {
        let mut source = view();
        let mut fields = fields_of(&source);
        source.runtime.as_mut().unwrap().status = status;
        normalize(
            &mut fields,
            source.agent.as_ref().unwrap(),
            &source,
            false,
            ProjectionPolicy::HOOK_AUTHORITATIVE,
        );
        let active = matches!(
            status,
            TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
        );
        assert_eq!(
            fields.lifecycle,
            if active {
                Lifecycle::Running {}
            } else {
                Lifecycle::Unknown {}
            }
        );
        assert_eq!(
            fields.freshness,
            if active {
                Freshness::Current
            } else {
                Freshness::Stale
            }
        );
        assert_eq!(
            fields.reachability,
            if status == TerminalRuntimeStatus::Unreachable {
                Reachability::Unreachable
            } else {
                Reachability::Reachable
            }
        );
    }
    for absent in [true, false] {
        let mut source = view();
        let mut fields = fields_of(&source);
        if absent {
            source.runtime = None;
        } else {
            source.runtime.as_mut().unwrap().runtime_epoch = Uuid::new_v4();
        }
        normalize(
            &mut fields,
            source.agent.as_ref().unwrap(),
            &source,
            false,
            ProjectionPolicy::HOOK_AUTHORITATIVE,
        );
        assert_eq!(fields.lifecycle, Lifecycle::Unknown {});
        assert_eq!(fields.activity, Activity::Unknown);
        assert_eq!(fields.reachability, Reachability::Unreachable);
    }
}

#[test]
fn unknown_recovery_and_process_observer_policy_are_not_promoted_to_hooks() {
    for recovery in [
        AgentRecoveryAvailability::Unavailable,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ] {
        let mut source = view();
        source.agent.as_mut().unwrap().recovery_availability = recovery;
        let fields = normalized(&source, false, ProjectionPolicy::HOOK_AUTHORITATIVE);
        assert_eq!(fields.source, ObservationSource::ProcessObserver);
        assert_eq!(fields.activity, Activity::Unknown);
        assert_eq!(fields.recovery, RecoveryStatus::Unavailable);
    }
    let mut source = view();
    source.manifest.recovery_state = RecoveryState::Uncertain;
    let fields = normalized(&source, false, ProjectionPolicy::HOOK_AUTHORITATIVE);
    assert_eq!(fields.freshness, Freshness::Stale);
    assert_eq!(fields.recovery, RecoveryStatus::Unavailable);
    source.manifest.recovery_state = RecoveryState::Created;
    source.agent.as_mut().unwrap().kind = AgentKind::Other(String::from("other"));
    let fields = normalized(&source, false, ProjectionPolicy::PROCESS_OBSERVER);
    assert_eq!(fields.source, ObservationSource::ProcessObserver);
    assert_eq!(fields.activity, Activity::Unknown);
}

#[test]
fn confirmed_agent_exit_never_borrows_shell_exit_code() {
    let mut source = view();
    exit(&mut source);
    let fields = normalized(&source, false, ProjectionPolicy::HOOK_AUTHORITATIVE);
    assert_eq!(fields.lifecycle, Lifecycle::Exited { exit_code: None });
    assert_eq!(fields.activity, Activity::Idle);
    assert_eq!(fields.freshness, Freshness::Current);
}
