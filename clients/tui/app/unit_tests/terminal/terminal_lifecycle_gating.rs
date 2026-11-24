use super::{fixtures, query_support, startup_support};
use crate::{StartupAction, network::Job};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};

const INPUT: &[u8] = b"terminal-input";

fn changed(terminal: &TerminalSummary, status: RuntimeStatus) -> TerminalSummary {
    let mut fields = terminal.fields().clone();
    fields.runtime.as_mut().unwrap().status = status;
    TerminalSummary::new(fields).unwrap()
}

#[test]
fn non_running_terminal_cannot_acquire_takeover_write_or_renew() {
    for status in [
        RuntimeStatus::Starting,
        RuntimeStatus::Finalizing,
        RuntimeStatus::OutputBlocked,
        RuntimeStatus::Exited,
        RuntimeStatus::Lost,
        RuntimeStatus::Unreachable,
    ] {
        let terminal = changed(&fixtures::session(), status);
        let address = terminal.fields().address;
        let mut state = crate::client_capability_support::state();
        state.control_generation = Some(uuid::Uuid::new_v4());
        state.replace_terminals(vec![terminal.clone()]);
        state.bind_focused(Some(address)).unwrap();
        state.record_grant(fixtures::grant(&terminal), std::time::Instant::now());
        let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
        for takeover in [false, true] {
            crate::polling::acquire(&mut state, &jobs, takeover);
        }
        assert!(!crate::terminal_actions::deliver(
            &mut state,
            &jobs,
            Ok(INPUT.to_vec())
        ));
        assert!(incoming.try_recv().is_err());
        assert_eq!(
            state.status,
            crate::terminal_lifecycle::input_blocked_hint(&state, address).unwrap()
        );
        crate::polling::renew_leases(&mut state, &jobs);
        assert!(state.writers.is_empty());
        assert!(matches!(
            incoming.try_recv().unwrap(),
            Job::Call {
                command: Command::Release { .. },
                ..
            }
        ));
        assert!(incoming.try_recv().is_err());
    }
}

#[test]
fn automatic_input_waits_until_starting_terminal_is_running() {
    let mut fixture = startup_support::Fixture::new(StartupAction::Attach { selector: None });
    let starting = startup_support::session(fixture.hub, RuntimeStatus::Starting);
    fixture.ready();
    fixture.state.replace_terminals(vec![starting.clone()]);
    fixture.apply();
    fixture.no_jobs();
    assert!(fixture.state.initial_control.is_some());
    let running = changed(&starting, RuntimeStatus::Running);
    fixture.state.replace_terminals(vec![running.clone()]);
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    fixture.acquired(&running);
}

#[test]
fn observed_terminal_exit_fences_late_grants_and_takeover_without_erasing_output() {
    let mut fixture = query_support::Fixture::new();
    let before = fixture.state.surfaces[&fixture.target.address]
        .screen()
        .contents();
    fixture
        .state
        .mark_ended(fixture.target.address, fixture.target.runtime_epoch);
    fixture.state.retire_writer(fixture.target.address);
    crate::lease_events::observed(&mut fixture.state, &fixture.jobs, fixture.target, None);
    fixture.no_jobs();
    crate::lease_events::granted(&mut fixture.state, &fixture.jobs, fixture.grant);
    assert!(fixture.state.writers.is_empty());
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Call {
            command: Command::Release { .. },
            ..
        }
    ));
    crate::observation_events::closed(&mut fixture.state, fixture.target, None);
    assert!(!fixture.state.is_offline(fixture.target.address));
    assert_eq!(
        fixture.state.surfaces[&fixture.target.address]
            .screen()
            .contents(),
        before
    );
    assert_eq!(fixture.state.status, yolo_tui_lifecycle::ENDED_HINT);
}

#[test]
fn clean_output_close_preserves_healthy_authority_but_blocks_input_until_reconnected() {
    let mut fixture = query_support::Fixture::new();
    let before = fixture.state.surfaces[&fixture.target.address]
        .screen()
        .contents();
    crate::observation_events::closed(&mut fixture.state, fixture.target, None);
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    assert!(!fixture.state.input_ready(fixture.target.address));
    assert!(fixture.state.is_offline(fixture.target.address));
    assert!(!fixture.state.ended_epoch(fixture.target.address).is_some());
    assert_eq!(
        fixture.state.surfaces[&fixture.target.address]
            .screen()
            .contents(),
        before
    );
    assert_eq!(
        fixture.state.status,
        yolo_tui_lifecycle::TERMINAL_RECONNECTING_LABEL
    );
}
