use super::{
    query_support::Fixture, terminal_input_navigation_support::*, terminal_input_ordering::queued,
};
use crate::{Mode, terminal_actions, terminal_input_authority};
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::Reply;

#[test]
fn unknown_background_input_requires_explicit_recovery_after_navigation() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    navigate(&mut fixture);
    fixture.finish(request, unknown(), true);
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    fixture.state.mode = Mode::Terminal;
    super::terminal_input_automatic_loss::late_renewal(&mut fixture);
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(SUFFIX.to_vec())
    ));
    fixture.no_jobs();
}

#[test]
fn return_view_keeps_navigation_live_while_the_prefix_outcome_is_pending() {
    let mut fixture = Fixture::new();
    let prefix = queued(&mut fixture, PREFIX);
    navigate(&mut fixture);
    let current = queued(&mut fixture, SUFFIX);
    assert!(fixture.state.inputs.can_read());
    assert!(!fixture.state.inputs.waiting());
    assert!(current.validity.as_ref().unwrap().is_current());
    crate::terminal_actions::prefix(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        crossterm::event::KeyCode::Char(crate::help_constants::SESSIONS_KEY),
    );
    assert_eq!(fixture.state.mode, Mode::Inventory);
    fixture.state.sync_query_authority();
    fixture.finish(prefix, unknown(), true);
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    assert!(!current.validity.as_ref().unwrap().is_current());
    fixture.finish(current, Err(ControlError::Invalidated), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}

#[test]
fn exact_success_or_navigation_unsent_outcomes_preserve_the_new_view_suffix() {
    for outcome in [
        Some(Ok(Reply::Input { duplicate: false })),
        Some(Err(ControlError::Invalidated)),
        None,
    ] {
        let mut fixture = Fixture::new();
        let prefix = queued(&mut fixture, PREFIX);
        navigate(&mut fixture);
        let current = queued(&mut fixture, SUFFIX);
        if let Some(outcome) = outcome {
            fixture.finish(prefix, outcome, true);
        } else {
            crate::events::apply(
                &mut fixture.state,
                &fixture.jobs,
                crate::network::Update::NotSent { request: prefix },
            );
        }
        assert!(!terminal_input_authority::blocked(
            &fixture.state,
            fixture.target.address
        ));
        assert!(current.validity.as_ref().unwrap().is_current());
        fixture.finish(current, Ok(Reply::Input { duplicate: false }), true);
        assert!(fixture.state.inputs.pending.is_empty());
        fixture.no_jobs();
    }
}

#[test]
fn a_tampered_or_duplicate_old_receipt_cannot_poison_current_stream() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, PREFIX);
    navigate(&mut fixture);
    let current = queued(&mut fixture, SUFFIX);
    let mut forged = original.clone();
    forged.validity = current.validity.clone();
    fixture.finish(forged, unknown(), false);
    assert!(current.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.control_generation.is_some());
    fixture.finish(
        original.clone(),
        Ok(Reply::Input { duplicate: false }),
        true,
    );
    fixture.finish(original, unknown(), true);
    assert!(current.validity.as_ref().unwrap().is_current());
    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    fixture.no_jobs();
}

#[test]
fn fully_settled_background_streams_are_released_but_active_view_queues_remain_valid() {
    let mut fixture = Fixture::new();
    let first = queued(&mut fixture, PREFIX);
    fixture.finish(first, Ok(Reply::Input { duplicate: false }), true);
    assert!(
        fixture
            .state
            .inputs
            .streams
            .contains_key(&fixture.target.address)
    );
    let second = queued(&mut fixture, SUFFIX);
    assert!(second.validity.as_ref().unwrap().is_current());
    fixture.finish(second, Ok(Reply::Input { duplicate: false }), true);
    navigate(&mut fixture);
    fixture.state.sync_query_authority();
    assert!(fixture.state.inputs.streams.is_empty());
    let returned = queued(&mut fixture, SUFFIX);
    assert!(returned.validity.as_ref().unwrap().is_current());
    fixture.no_jobs();
}
