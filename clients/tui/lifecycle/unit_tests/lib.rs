#[path = "../test_support/fixtures.rs"]
mod fixtures;
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, Freshness, Lifecycle, Reachability};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::RuntimeStatus;
use yolo_tui_lifecycle::*;

#[test]
fn runtime_lifecycle_never_infers_exit_from_absence_or_reachability() {
    let mut terminal = fixtures::terminal();
    for (status, expected) in [
        (RuntimeStatus::Starting, TerminalPhase::Starting),
        (RuntimeStatus::Running, TerminalPhase::Running),
        (RuntimeStatus::Finalizing, TerminalPhase::Finalizing),
        (RuntimeStatus::OutputBlocked, TerminalPhase::OutputBlocked),
        (RuntimeStatus::Exited, TerminalPhase::Ended),
        (RuntimeStatus::Lost, TerminalPhase::Lost),
        (RuntimeStatus::Unreachable, TerminalPhase::Unreachable),
    ] {
        terminal.runtime.as_mut().unwrap().status = status;
        assert_eq!(terminal_phase(&terminal, None), expected);
        assert_eq!(expected.allows_input(), status == RuntimeStatus::Running);
        assert!(!expected.label().is_empty());
        assert_eq!(
            expected.input_blocked_hint().is_none(),
            expected.allows_input()
        );
    }
    terminal.runtime = None;
    assert_eq!(terminal_phase(&terminal, None), TerminalPhase::Unavailable);
    terminal.archived = true;
    assert_eq!(terminal_phase(&terminal, None), TerminalPhase::Archived);
}

#[test]
fn archived_or_ended_terminal_cannot_display_a_cached_running_agent() {
    let mut terminal = fixtures::terminal();
    let agent = fixtures::agent(&mut terminal);
    for status in [
        RuntimeStatus::Exited,
        RuntimeStatus::Lost,
        RuntimeStatus::Unreachable,
    ] {
        terminal.runtime.as_mut().unwrap().status = status;
        assert!(active_agent(&terminal, Some(&agent), true).is_none());
    }
    terminal.runtime.as_mut().unwrap().status = RuntimeStatus::Running;
    terminal.archived = true;
    assert!(active_agent(&terminal, Some(&agent), true).is_none());
    terminal.archived = false;
    assert!(active_agent(&terminal, None, true).is_none());
    terminal.runtime = None;
    assert!(active_agent(&terminal, Some(&agent), true).is_none());
}

#[test]
fn old_runtime_exit_cannot_end_a_new_pty_incarnation() {
    let mut terminal = fixtures::terminal();
    let ended = terminal.runtime.unwrap().target.runtime_epoch;
    assert_eq!(terminal_phase(&terminal, Some(ended)), TerminalPhase::Ended);
    terminal.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    assert_eq!(
        terminal_phase(&terminal, Some(ended)),
        TerminalPhase::Running
    );
}

#[test]
fn provider_exit_returns_to_running_shell_without_an_active_agent_badge() {
    let mut terminal = fixtures::terminal();
    let running = fixtures::agent(&mut terminal);
    assert!(active_agent(&terminal, Some(&running), true).is_some());
    let mut fields = running.fields().clone();
    fields.lifecycle = Lifecycle::Exited {
        exit_code: Some(i32::default()),
    };
    fields.activity = Activity::Idle;
    let exited = AgentSnapshot::new(fields).unwrap();
    assert!(active_agent(&terminal, Some(&exited), true).is_none());
    assert_eq!(terminal_phase(&terminal, None), TerminalPhase::Running);
}

#[test]
fn badge_requires_current_invocation_and_exact_terminal_runtime() {
    let mut terminal = fixtures::terminal();
    let agent = fixtures::agent(&mut terminal);
    assert!(active_agent(&terminal, Some(&agent), false).is_none());
    for change in [
        Change::Stale,
        Change::Unreachable,
        Change::Unknown,
        Change::Runtime,
    ] {
        let mut fields = agent.fields().clone();
        match change {
            Change::Stale => fields.freshness = Freshness::Stale,
            Change::Unreachable => fields.reachability = Reachability::Unreachable,
            Change::Unknown => fields.lifecycle = Lifecycle::Unknown {},
            Change::Runtime => fields.terminal.runtime_epoch = RuntimeEpoch::new(),
        }
        let stale = AgentSnapshot::new(fields).unwrap();
        assert!(active_agent(&terminal, Some(&stale), true).is_none());
    }
    terminal.agent = None;
    assert!(active_agent(&terminal, Some(&agent), true).is_none());
}

enum Change {
    Stale,
    Unreachable,
    Unknown,
    Runtime,
}

#[test]
fn loss_of_transport_revokes_input_without_changing_runtime_lifecycle() {
    for connection in [
        ConnectionState::HubReconnecting,
        ConnectionState::TerminalReconnecting,
    ] {
        let projection = TerminalProjection::new(TerminalPhase::Running, connection, true, true);
        assert_eq!(projection.phase, TerminalPhase::Running);
        assert_eq!(projection.input, InputAccess::Unavailable);
        assert_eq!(Some(projection.label()), connection.label());
        assert!(projection.is_warning());
    }
}

#[test]
fn blocked_output_and_ended_terminal_are_distinct_from_disconnect_and_read_only() {
    let blocked = TerminalProjection::new(
        TerminalPhase::OutputBlocked,
        ConnectionState::TerminalReconnecting,
        true,
        true,
    );
    assert_eq!(blocked.label(), OUTPUT_BLOCKED_LABEL);
    assert_eq!(blocked.input, InputAccess::Unavailable);
    assert_eq!(blocked.action_hint(), Some(OUTPUT_BLOCKED_ACTION));
    let ended =
        TerminalProjection::new(TerminalPhase::Ended, ConnectionState::Connected, true, true);
    assert_eq!(ended.label(), ENDED_LABEL);
    assert_eq!(ended.input, InputAccess::Unavailable);
    assert_eq!(ended.action_hint(), Some(ENDED_ACTION));
}

#[test]
fn writer_authority_and_geometry_readiness_are_independent() {
    for (writer, ready, expected) in [
        (false, false, InputAccess::ReadOnly),
        (false, true, InputAccess::ReadOnly),
        (true, false, InputAccess::Synchronizing),
        (true, true, InputAccess::Writing),
    ] {
        let projection = TerminalProjection::new(
            TerminalPhase::Running,
            ConnectionState::Connected,
            writer,
            ready,
        );
        assert_eq!(projection.input, expected);
        assert_eq!(projection.phase, TerminalPhase::Running);
    }
}
