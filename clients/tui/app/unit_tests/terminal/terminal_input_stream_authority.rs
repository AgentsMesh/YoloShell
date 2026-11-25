use super::{
    query_support::Fixture, terminal_input_navigation_support::*, terminal_input_ordering::queued,
};
use crate::{Mode, terminal_input_authority};
use std::time::Instant;

#[test]
fn explicit_fresh_writer_replaces_the_obligation_before_an_old_unknown_reply() {
    for reconnect in [false, true] {
        let mut fixture = Fixture::new();
        let original = queued(&mut fixture, PREFIX);
        navigate(&mut fixture);
        if reconnect {
            fixture.state.disconnected();
            fixture.state.control_generation = Some(uuid::Uuid::new_v4());
            crate::client_capability_support::install(&mut fixture.state);
        }
        terminal_input_authority::request_resume(&mut fixture.state, fixture.target.address);
        fixture.state.sync_query_authority();
        let replacement = fixture.replace_writer();
        let current = queued(&mut fixture, SUFFIX);
        fixture.finish(original, unknown(), true);
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            replacement
        );
        assert!(current.validity.as_ref().unwrap().is_current());
        assert!(!terminal_input_authority::blocked(
            &fixture.state,
            fixture.target.address
        ));
    }
}

#[test]
fn ordinary_new_grant_cannot_hide_unsettled_background_input() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, PREFIX);
    navigate(&mut fixture);
    fixture.replace_writer();
    assert!(fixture.state.writers.is_empty());
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    fixture.finish(original, unknown(), true);
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
}

#[test]
fn renewing_background_writer_keeps_the_original_stream_obligation() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, PREFIX);
    navigate(&mut fixture);
    fixture.state.record_grant(fixture.grant, Instant::now());
    fixture.finish(original, unknown(), true);
    assert!(fixture.state.writers.is_empty());
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    fixture.state.mode = Mode::Terminal;
    assert_eq!(
        terminal_input_authority::status(&fixture.state),
        Some(terminal_input_authority::INTERRUPTED_HINT)
    );
}

#[test]
fn background_unknown_cannot_revoke_the_selected_other_terminal_or_its_status() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, PREFIX);
    let other = super::fixtures::session();
    let target = other.fields().runtime.unwrap().target;
    let mut fields = *super::fixtures::grant(&other).fields();
    fields.principal = fixture.state.principal;
    let other_grant = yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap();
    fixture.state.terminals.insert(target.address, other);
    fixture
        .state
        .tab_layout_mut()
        .split(
            yolo_presentation_layout::Axis::Horizontal,
            Some(target.address),
        )
        .unwrap();
    fixture.state.record_grant(other_grant, Instant::now());
    fixture.state.set_last_size(
        target.address,
        yolo_hub_terminal_protocol::TerminalSize::default(),
    );
    fixture.state.surfaces.insert(
        target.address,
        yolo_tui_terminal::TerminalSurface::new(&super::query_support::snapshot(
            target,
            super::query_support::INITIAL_SEQUENCE,
            super::query_support::POSITION,
        )),
    );
    fixture.state.sync_query_authority();
    let current = queued(&mut fixture, SUFFIX);
    let status = fixture.state.status.clone();

    fixture.finish(original, unknown(), true);

    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    assert_eq!(fixture.state.writers[&target.address].grant, other_grant);
    assert!(current.validity.as_ref().unwrap().is_current());
    assert_eq!(fixture.state.status, status);
}

#[test]
fn replacement_runtime_never_inherits_the_previous_runtime_input_obligation() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, PREFIX);
    navigate(&mut fixture);
    let mut terminal = fixture.state.terminals[&fixture.target.address]
        .fields()
        .clone();
    let runtime = terminal.runtime.as_mut().unwrap();
    runtime.target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    let target = runtime.target;
    fixture.state.terminals.insert(
        target.address,
        yolo_hub_terminal_protocol::TerminalSummary::new(terminal).unwrap(),
    );
    let mut fields = *fixture.grant.fields();
    fields.target = target;
    let replacement = yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap();
    fixture.state.record_grant(replacement, Instant::now());
    fixture.state.surfaces.insert(
        target.address,
        yolo_tui_terminal::TerminalSurface::new(&super::query_support::snapshot(
            target,
            super::query_support::INITIAL_SEQUENCE,
            super::query_support::POSITION,
        )),
    );
    fixture.ready_size();
    let current = queued(&mut fixture, SUFFIX);

    fixture.finish(original, unknown(), true);

    assert_eq!(fixture.state.writers[&target.address].grant, replacement);
    assert!(current.validity.as_ref().unwrap().is_current());
    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        target.address
    ));
}
