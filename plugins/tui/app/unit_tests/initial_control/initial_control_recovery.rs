use super::{
    fixtures,
    initial_control_late_grants::{finish, pending_acquire},
};
use crate::{
    control, events, initial_control,
    network::{Job, Update},
    polling,
};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlEvent, ControlRequest};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseMode, TerminalSummary};
use yolo_tui_terminal_control::RETRY_INTERVAL;

#[test]
fn first_acquisition_disconnect_preserves_only_intent_and_recovers_with_a_fresh_request() {
    let (mut fixture, original, grant) = pending_acquire();
    let old_generation = fixture.state.control_generation;
    initial_control::disconnected(&mut fixture.state);
    assert!(fixture.state.initial_control.is_none());
    assert!(!original.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.writers.is_empty());
    assert_eq!(
        control::label(&fixture.state, grant.fields().target.address),
        super::control_recovery_support::RESTORING_LABEL
    );
    fixture
        .state
        .control
        .disconnected(Instant::now() - RETRY_INTERVAL);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Unreachable {
            generation: old_generation,
            message: crate::constants::DISCONNECTED_LABEL.into(),
        }),
    );
    let generation = Uuid::new_v4();
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(crate::client_capability_support::ready(generation)),
    );
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    let Job::Acquire {
        generation: actual,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("expected recovered initial acquisition")
    };
    assert_eq!(actual, generation);
    assert_eq!(request.target, grant.fields().target);
    assert_eq!(request.mode, LeaseMode::Acquire {});
    assert!(validity.is_current());
    finish(&mut fixture, original, grant);
    assert!(fixture.state.writers.is_empty());
    finish(
        &mut fixture,
        ControlRequest {
            generation: Some(generation),
            destination: origin,
            command: Command::Lease { request },
            validity: Some(validity),
        },
        grant,
    );
    assert_eq!(
        fixture.state.writers[&grant.fields().target.address].grant,
        grant
    );
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn actual_disconnect_transfers_a_current_initial_intent_before_clearing_connection_identity() {
    let (mut fixture, original, grant) = pending_acquire();
    fixture.state.disconnected();
    assert!(!original.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.control_generation.is_none());
    assert!(fixture.state.initial_control.is_none());
    assert_eq!(
        control::label(&fixture.state, grant.fields().target.address),
        super::control_recovery_support::RESTORING_LABEL
    );
    assert!(fixture.state.writers.is_empty());
}

#[test]
fn navigation_or_expired_initial_intent_never_becomes_persistent_control_on_disconnect() {
    for cancelled in [true, false] {
        let (mut fixture, original, grant) = pending_acquire();
        if cancelled {
            initial_control::user_event(&mut fixture.state);
        } else {
            fixture.state.initial_control.as_mut().unwrap().deadline = Instant::now();
        }
        fixture.state.disconnected();
        assert!(!original.validity.as_ref().unwrap().is_current());
        assert_eq!(
            control::label(&fixture.state, grant.fields().target.address),
            crate::constants::READ_ONLY_LABEL
        );
        assert!(fixture.state.writers.is_empty());
    }
}

#[test]
fn changed_runtime_binding_or_connection_never_inherits_the_old_initial_intent() {
    for change in [Change::Runtime, Change::Binding, Change::Connection] {
        let (mut fixture, _, grant) = pending_acquire();
        let target = grant.fields().target;
        match change {
            Change::Runtime => {
                let mut fields = fixture.state.terminals[&target.address].fields().clone();
                fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
                fixture
                    .state
                    .replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
            }
            Change::Binding => {
                let other = fixtures::session();
                fixture
                    .state
                    .tab_layout_mut()
                    .bind(fixture.pane, Some(other.fields().address))
                    .unwrap();
            }
            Change::Connection => fixture.state.control_generation = Some(Uuid::new_v4()),
        }
        fixture.state.disconnected();
        assert_eq!(
            control::label(&fixture.state, target.address),
            crate::constants::READ_ONLY_LABEL
        );
        assert!(fixture.state.writers.is_empty());
    }
}

enum Change {
    Runtime,
    Binding,
    Connection,
}
