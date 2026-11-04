use super::{query_support::Fixture, terminal_input_ordering::queued};
use crate::Mode;
use yolo_hub_client_channels::{ControlError, ControlRequest, RequestScope};

const ORIGINAL: &[u8] = b"original";
const REPLACEMENT: &[u8] = b"replacement";

#[test]
fn tampered_scope_generation_or_destination_cannot_consume_an_input_receipt() {
    let tamper: &[fn(&mut ControlRequest)] = &[
        |request| request.generation = Some(uuid::Uuid::new_v4()),
        |request| request.destination = yolo_hub_identity::HubId::new(),
        |request| request.validity = None,
        |request| {
            request.validity = Some(
                RequestScope::ordered_inputs()
                    .validity_until(request.validity.as_ref().unwrap().deadline()),
            );
        },
    ];
    for tamper in tamper {
        let mut fixture = Fixture::new();
        let original = queued(&mut fixture, ORIGINAL);
        let mut changed = original.clone();
        tamper(&mut changed);
        let generation = fixture.state.control_generation;
        fixture.finish(changed, Err(ControlError::Invalidated), false);
        assert_eq!(fixture.state.control_generation, generation);
        assert!(!fixture.state.inputs.pending.is_empty());
        assert!(original.validity.as_ref().unwrap().is_current());
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            fixture.grant
        );
        fixture.finish(original, Err(ControlError::Invalidated), true);
        assert!(fixture.state.inputs.pending.is_empty());
    }
}

#[test]
fn a_same_scope_but_different_deadline_cannot_consume_an_input_receipt() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, ORIGINAL);
    let mut fields = *fixture.grant.fields();
    fields.ttl_ms += super::query_support::LEASE_EXTENSION_MS;
    let renewed = yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap();
    fixture
        .state
        .record_grant(renewed, std::time::Instant::now());
    let current = queued(&mut fixture, REPLACEMENT);
    let mut changed = original.clone();
    changed.validity = current.validity.clone();
    let generation = fixture.state.control_generation;
    fixture.finish(changed, Err(ControlError::Invalidated), false);
    assert_eq!(fixture.state.control_generation, generation);
    assert!(original.validity.as_ref().unwrap().is_current());
    assert!(current.validity.as_ref().unwrap().is_current());
    assert!(
        fixture
            .state
            .inputs
            .pending
            .contains_key(match &original.command {
                yolo_hub_rpc_protocol::Command::Input { request } => &request.fields().request_id,
                _ => panic!(),
            })
    );
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        renewed
    );
}

#[test]
fn borrowing_a_new_scopes_validity_cannot_make_an_old_receipt_revoke_the_current_writer() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, ORIGINAL);
    fixture.state.mode = Mode::Inventory;
    fixture.state.sync_query_authority();
    let current = queued(&mut fixture, REPLACEMENT);
    let mut changed = original.clone();
    changed.validity = current.validity.clone();
    let generation = fixture.state.control_generation;
    fixture.finish(changed, Err(ControlError::Invalidated), false);
    assert_eq!(fixture.state.control_generation, generation);
    assert!(current.validity.as_ref().unwrap().is_current());
    fixture.finish(original, Err(ControlError::Invalidated), true);
    assert!(current.validity.as_ref().unwrap().is_current());
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
}
