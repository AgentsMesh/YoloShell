use super::fixtures;
use crate::{
    State, events,
    network::{Job, Update},
};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError, SecretCapability};
use yolo_presentation_layout::Axis;

const NEXT_LEASE_GENERATION: u64 = yolo_hub_terminal_protocol::FIRST_LEASE_GENERATION + 1;

fn writers() -> (State, LeaseGrant, LeaseGrant) {
    let session = fixtures::session();
    let other = fixtures::session();
    let grant = fixtures::grant(&session);
    let other_grant = fixtures::grant(&other);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session, other]);
    state
        .bind_focused(Some(grant.fields().target.address))
        .unwrap();
    state
        .tab_layout_mut()
        .split(Axis::Horizontal, Some(other_grant.fields().target.address))
        .unwrap();
    for writer in [grant, other_grant] {
        state.record_grant(writer, Instant::now());
        state.mark_renewal_pending(writer.fields().target.address);
    }
    (state, grant, other_grant)
}

fn reject(state: &mut State, jobs: &mpsc::Sender<Job>, grant: LeaseGrant, command: Command) {
    let generation = state.control_generation;
    let error = ControlError::Client(yolo_hub_client::ClientError::Remote(
        PublicError::InvalidLease,
    ));
    let message = error.to_string();
    events::apply(
        state,
        jobs,
        Update::Control(ControlEvent::Finished {
            request: ControlRequest {
                validity: None,
                generation,
                destination: grant.fields().target.address.hub_id,
                command,
            },
            result: Err(error),
            connected: true,
        }),
    );
    assert_eq!(state.control_generation, generation);
    assert_eq!(state.status, message);
}

#[test]
fn failed_current_renew_or_release_retires_only_the_matching_writer() {
    for release in [false, true] {
        let (mut state, grant, other) = writers();
        let address = grant.fields().target.address;
        let other_address = other.fields().target.address;
        let command = if release {
            Command::Release { grant }
        } else {
            Command::Renew { grant }
        };
        let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
        reject(&mut state, &jobs, grant, command);
        assert!(!state.writers.contains_key(&address));
        assert!(!state.renewal_pending(address));
        assert_eq!(state.writers[&other_address].grant, other);
        assert!(state.renewal_pending(other_address));
        assert!(incoming.try_recv().is_err());
    }
}

#[test]
fn failed_old_renew_or_release_preserves_a_replacement_and_its_pending_renewal() {
    for release in [false, true] {
        let (mut state, grant, other) = writers();
        let address = grant.fields().target.address;
        let other_address = other.fields().target.address;
        let mut replacement = *grant.fields();
        replacement.generation = NEXT_LEASE_GENERATION;
        replacement.capability = SecretCapability::generate();
        let replacement = LeaseGrant::new(replacement).unwrap();
        state.record_grant(replacement, Instant::now());
        state.mark_renewal_pending(address);
        let command = if release {
            Command::Release { grant }
        } else {
            Command::Renew { grant }
        };
        let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
        reject(&mut state, &jobs, grant, command);
        assert_eq!(state.writers[&address].grant, replacement);
        assert!(state.renewal_pending(address));
        assert_eq!(state.writers[&other_address].grant, other);
        assert!(state.renewal_pending(other_address));
        assert!(incoming.try_recv().is_err());
    }
}
