use super::fixtures;
use crate::{State, events, network::Update};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, SecretCapability};

const NEXT_LEASE_GENERATION: u64 = yolo_hub_terminal_protocol::FIRST_LEASE_GENERATION + 1;

fn writer() -> (State, LeaseGrant) {
    let terminal = fixtures::session();
    let grant = fixtures::grant(&terminal);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![terminal]);
    state
        .bind_focused(Some(grant.fields().target.address))
        .unwrap();
    state.record_grant(grant, Instant::now());
    state.mark_renewal_pending(grant.fields().target.address);
    (state, grant)
}

fn renewed(grant: LeaseGrant) -> LeaseGrant {
    let mut fields = *grant.fields();
    fields.expires_at_ms += fixtures::TTL_MS as i64;
    LeaseGrant::new(fields).unwrap()
}

#[test]
fn late_successful_renewal_preserves_replacement_and_its_pending_renewal() {
    let (mut state, original) = writer();
    let address = original.fields().target.address;
    let mut replacement = *original.fields();
    replacement.generation = NEXT_LEASE_GENERATION;
    replacement.capability = SecretCapability::generate();
    let replacement = LeaseGrant::new(replacement).unwrap();
    state.record_grant(replacement, Instant::now());
    state.writers.get_mut(&address).unwrap().renew = Instant::now();
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let stale = renewed(original);
    events::apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Renew { grant: original },
            reply: Reply::Lease { grant: stale },
        },
    );
    assert_eq!(state.writers[&address].grant, replacement);
    assert!(state.renewal_pending(address));
    assert!(
        incoming.try_recv().is_err(),
        "stale renewal emitted a command"
    );
    crate::polling::renew_leases(&mut state, &jobs);
    assert!(
        incoming.try_recv().is_err(),
        "stale receipt queued a duplicate renewal"
    );
}

#[test]
fn current_successful_renewal_clears_only_its_own_pending_marker() {
    let (mut state, original) = writer();
    let address = original.fields().target.address;
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let current = renewed(original);
    events::apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Renew { grant: original },
            reply: Reply::Lease { grant: current },
        },
    );
    assert_eq!(state.writers[&address].grant, current);
    assert!(!state.renewal_pending(address));
    assert!(incoming.try_recv().is_err());
}
