use super::fixtures::*;
use yolo_hub_local_platform::{recovery_hint, safe_termination};
use yolo_agent_contract::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_hub_terminal_protocol::TerminalRecoveryHint;
use yolo_runtime::{
    model::{BootstrapDelivery, RecoveryState},
    protocol::capability,
};

fn binding(availability: AgentRecoveryAvailability) -> AgentBinding {
    use super::fixtures::constants::*;
    AgentBinding {
        schema_version: SCHEMA.into(),
        terminal_id: target().address.terminal_id.as_uuid(),
        kind: AgentKind::Codex,
        provider_session_id: NAME.into(),
        executable: SHELL.into(),
        cwd: DIRECTORY.into(),
        evidence_path: DIRECTORY.into(),
        data_home: DIRECTORY.into(),
        process_pid: None,
        process_start_time: None,
        state: AgentState::Unknown,
        recovery_availability: availability,
        detected_at_ms: CLOCK,
        updated_at_ms: CLOCK,
    }
}

#[test]
fn capability_projection_preserves_unknown_supported_and_unsupported() {
    let mut view = view();
    assert_eq!(safe_termination(&view), None);
    view.runtime.as_mut().unwrap().capabilities = Some(vec![]);
    assert_eq!(safe_termination(&view), Some(false));
    view.runtime.as_mut().unwrap().capabilities =
        Some(vec![super::fixtures::constants::NAME.into()]);
    assert_eq!(safe_termination(&view), Some(false));
    view.runtime.as_mut().unwrap().capabilities =
        Some(vec![capability::SAFE_PROCESS_TREE_TERMINATION.into()]);
    assert_eq!(safe_termination(&view), Some(true));
    view.runtime = None;
    assert_eq!(safe_termination(&view), None);
}

#[test]
fn hints_preserve_live_survivors_and_never_invent_agent_resume_without_availability() {
    let mut view = view();
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Ready);
    view.manifest.recovery_state = RecoveryState::Restoring;
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Recovering);
    view.manifest.recovery_state = RecoveryState::NeedsRecreate;
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Recreate);
    view.manifest.recovery_state = RecoveryState::Uncertain;
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Recreate);
    view.runtime.as_mut().unwrap().bootstrap_delivery = BootstrapDelivery::Survivor;
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Ready);
    view.runtime.as_mut().unwrap().bootstrap_delivery = BootstrapDelivery::NotRequired;
    view.runtime.as_mut().unwrap().capabilities = Some(vec![]);
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Ready);
    view.runtime = None;
    for (availability, hint) in [
        (
            AgentRecoveryAvailability::Available,
            TerminalRecoveryHint::Resume,
        ),
        (
            AgentRecoveryAvailability::Unavailable,
            TerminalRecoveryHint::Unavailable,
        ),
        (
            AgentRecoveryAvailability::UnmanagedLaunch,
            TerminalRecoveryHint::Unavailable,
        ),
    ] {
        view.agent = Some(binding(availability));
        assert_eq!(recovery_hint(&view), hint);
    }
    view.agent = None;
    view.manifest.recovery_state = RecoveryState::Created;
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Recreate);
    view.manifest.archived = true;
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Unavailable);
}

#[test]
fn exited_runtime_uses_provider_history_or_explicit_shell_recreation() {
    let mut view = view();
    view.runtime.as_mut().unwrap().status = yolo_runtime::model::TerminalRuntimeStatus::Exited;
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Recreate);
    view.agent = Some(binding(AgentRecoveryAvailability::Available));
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Resume);
    view.agent = Some(binding(AgentRecoveryAvailability::Unavailable));
    assert_eq!(recovery_hint(&view), TerminalRecoveryHint::Unavailable);
}
