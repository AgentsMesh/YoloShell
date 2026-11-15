use super::fixtures::{agent_control::host_view, target, view};
use crate::connector_admission::admission;
use yolo_agent_contract::AgentState;
use yolo_hub_connector_registry::RuntimeAdmission;
use yolo_runtime::model::{RecoveryState, TerminalRuntimeStatus};

#[test]
fn only_exact_live_certain_shell_without_native_owner_admits_a_connector() {
    let mut plain = view();
    for status in [
        TerminalRuntimeStatus::Running,
        TerminalRuntimeStatus::OutputBlocked,
    ] {
        plain.runtime.as_mut().unwrap().status = status;
        assert!(
            matches!(admission(target(), &plain), RuntimeAdmission::Available(observed) if observed == target())
        );
    }
    for state in [
        AgentState::Running,
        AgentState::WaitingForInput,
        AgentState::WaitingForPermission,
        AgentState::Unknown,
    ] {
        let mut native = host_view();
        native.agent.as_mut().unwrap().state = state;
        assert!(matches!(
            admission(target(), &native),
            RuntimeAdmission::NativeOwned
        ));
    }
    let mut historical = host_view();
    historical.agent.as_mut().unwrap().state = AgentState::Exited;
    assert!(matches!(
        admission(target(), &historical),
        RuntimeAdmission::Available(_)
    ));
}

#[test]
fn missing_unreachable_uncertain_or_substituted_host_evidence_is_unavailable() {
    let mut missing = view();
    missing.runtime = None;
    assert!(matches!(
        admission(target(), &missing),
        RuntimeAdmission::Unavailable
    ));
    for status in [
        TerminalRuntimeStatus::Starting,
        TerminalRuntimeStatus::Finalizing,
        TerminalRuntimeStatus::Unreachable,
    ] {
        let mut current = view();
        current.runtime.as_mut().unwrap().status = status;
        assert!(matches!(
            admission(target(), &current),
            RuntimeAdmission::Unavailable
        ));
    }
    let mut uncertain = view();
    uncertain.manifest.recovery_state = RecoveryState::Uncertain;
    assert!(matches!(
        admission(target(), &uncertain),
        RuntimeAdmission::Unavailable
    ));
    let mut substituted = view();
    substituted.manifest.terminal_id = uuid::Uuid::new_v4();
    assert!(matches!(
        admission(target(), &substituted),
        RuntimeAdmission::Unavailable
    ));
}

#[test]
fn positive_epoch_change_archive_or_exit_permanently_retires_the_old_scope() {
    let mut changed = view();
    changed.runtime.as_mut().unwrap().runtime_epoch = uuid::Uuid::new_v4();
    assert!(matches!(
        admission(target(), &changed),
        RuntimeAdmission::Retired
    ));
    let mut archived = view();
    archived.manifest.archived = true;
    assert!(matches!(
        admission(target(), &archived),
        RuntimeAdmission::Retired
    ));
    for status in [TerminalRuntimeStatus::Exited] {
        let mut ended = view();
        ended.runtime.as_mut().unwrap().status = status;
        assert!(matches!(
            admission(target(), &ended),
            RuntimeAdmission::Retired
        ));
    }
}
