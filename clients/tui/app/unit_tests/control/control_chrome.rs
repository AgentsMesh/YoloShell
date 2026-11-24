use super::query_support::Fixture;
use crate::control;
use std::time::{SystemTime, UNIX_EPOCH};
use yolo_hub_terminal_protocol::{LeaseObservation, LeaseObservationFields};
use yolo_tui_inventory::ControlBadge;

fn observed(fixture: &Fixture) -> LeaseObservation {
    LeaseObservation::new(LeaseObservationFields {
        target: fixture.target,
        generation: fixture.grant.fields().generation,
        holder_id: "other-client".into(),
        controller_instance_id: None,
        expires_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            + 60_000,
    })
    .unwrap()
}

#[test]
fn detail_distinguishes_local_control_from_read_only_takeover() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    assert_eq!(control::detail(&fixture.state, address), "Control: this UI");
    assert_eq!(
        control::badge(&fixture.state, address),
        ControlBadge::Writable
    );
    assert_eq!(
        control::marker(&fixture.state, address),
        Some(ControlBadge::Writable.glyph())
    );

    fixture.state.writers.clear();
    fixture.state.control.observe(fixture.target);
    fixture
        .state
        .control
        .record_observation(fixture.target, Some(&observed(&fixture)));
    let detail = control::detail(&fixture.state, address);
    assert!(detail.starts_with("Read Only"));
    assert!(detail.contains("Ctrl-b t"));
}

#[test]
fn compact_control_status_has_no_takeover_instruction() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    fixture.state.writers.clear();
    fixture.state.control.observe(fixture.target);
    fixture
        .state
        .control
        .record_observation(fixture.target, Some(&observed(&fixture)));
    assert_eq!(
        control::badge(&fixture.state, address),
        ControlBadge::ReadOnly
    );
    assert_eq!(
        control::marker(&fixture.state, address),
        Some(ControlBadge::ReadOnly.glyph())
    );
}

#[test]
fn observed_same_client_is_not_presented_as_an_unrelated_holder() {
    let mut fixture = Fixture::new();
    let fields = yolo_hub_terminal_protocol::LeaseObservationFields {
        target: fixture.target,
        generation: fixture.grant.fields().generation,
        holder_id: fixture.state.principal.client_id.to_string(),
        controller_instance_id: Some(fixture.state.principal.controller_instance_id.to_string()),
        expires_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            + 60_000,
    };
    let lease = LeaseObservation::new(fields).unwrap();
    fixture.state.writers.clear();
    fixture.state.control.observe(fixture.target);
    fixture
        .state
        .control
        .record_observation(fixture.target, Some(&lease));
    assert!(control::detail(&fixture.state, fixture.target.address).contains("this UI instance"));
}

#[test]
fn known_competing_owner_is_visible_while_write_intent_waits_for_it() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    fixture.state.control.observe(fixture.target);
    fixture
        .state
        .control
        .record_observation(fixture.target, Some(&observed(&fixture)));
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let detail = control::detail(&fixture.state, fixture.target.address);
    assert!(detail.contains("Read Only"));
    assert!(detail.contains("other-client"));
    assert!(detail.contains("Ctrl-b t"));
    assert!(!detail.contains("Restoring input"));
}
