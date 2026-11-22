use super::query_support::{Fixture, LEASE_EXTENSION_MS};
use crate::{Mode, network::Job, terminal_actions};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_terminal_protocol::LeaseGrant;

const PREFIX: &[u8] = b"printf '%s' 'complete";
const SUFFIX: &[u8] = b"'\r";

pub(super) fn queued(fixture: &mut Fixture, bytes: &[u8]) -> ControlRequest {
    fixture.state.mode = Mode::Terminal;
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(bytes.to_vec())
    ));
    let Job::UserInput(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    request
}

#[test]
fn rejected_prefix_after_renewal_cancels_newer_suffix_and_requires_fresh_writer_authority() {
    let mut fixture = Fixture::new();
    let prefix = queued(&mut fixture, PREFIX);
    let mut fields = *fixture.grant.fields();
    fields.ttl_ms += LEASE_EXTENSION_MS;
    fields.expires_at_ms += LEASE_EXTENSION_MS as i64;
    fixture
        .state
        .record_grant(LeaseGrant::new(fields).unwrap(), Instant::now());
    let suffix = queued(&mut fixture, SUFFIX);
    let original = prefix.validity.as_ref().unwrap();
    let later = suffix.validity.as_ref().unwrap();
    assert!(original.same_scope(later));
    assert!(original.deadline() < later.deadline());
    assert_eq!(original.admission_deadline(), later.admission_deadline());
    fixture.finish(prefix, Err(ControlError::Invalidated), true);
    assert!(!later.is_current());
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(SUFFIX.to_vec())
    ));
    fixture.finish(suffix, Err(ControlError::Invalidated), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}

#[test]
fn navigation_cancellation_keeps_the_background_writer_without_changing_its_status() {
    let mut fixture = Fixture::new();
    let prefix = queued(&mut fixture, PREFIX);
    fixture.state.mode = Mode::Inventory;
    fixture.state.sync_query_authority();
    let status = fixture.state.status.clone();
    fixture.finish(prefix, Err(ControlError::Invalidated), true);
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    assert_eq!(fixture.state.status, status);
    fixture.no_jobs();
}

#[test]
fn returning_to_the_same_writer_uses_a_new_view_that_known_navigation_cancellation_cannot_revoke() {
    let mut fixture = Fixture::new();
    let prefix = queued(&mut fixture, PREFIX);
    fixture.state.mode = Mode::Inventory;
    fixture.state.sync_query_authority();
    let current = queued(&mut fixture, SUFFIX);
    assert!(
        !prefix
            .validity
            .as_ref()
            .unwrap()
            .same_scope(current.validity.as_ref().unwrap())
    );
    fixture.finish(prefix, Err(ControlError::Invalidated), true);
    assert!(current.validity.as_ref().unwrap().is_current());
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    fixture.no_jobs();
}

#[test]
fn old_generation_and_replacement_writer_failures_cannot_revoke_current_input() {
    for reconnect in [false, true] {
        let mut fixture = Fixture::new();
        let previous = queued(&mut fixture, PREFIX);
        if reconnect {
            fixture.state.disconnected();
            fixture.state.control_generation = Some(uuid::Uuid::new_v4());
            crate::client_capability_support::install(&mut fixture.state);
        }
        crate::terminal_input_authority::request_resume(&mut fixture.state, fixture.target.address);
        let replacement = fixture.replace_writer();
        let current = queued(&mut fixture, SUFFIX);
        fixture.finish(previous, Err(ControlError::Invalidated), true);
        assert!(current.validity.as_ref().unwrap().is_current());
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            replacement
        );
        fixture.no_jobs();
    }
}

#[test]
fn a_known_unsent_prefix_cancels_the_group_before_the_next_local_input() {
    let mut fixture = Fixture::new();
    let prefix = queued(&mut fixture, PREFIX);
    let suffix = queued(&mut fixture, SUFFIX);
    crate::events::apply(
        &mut fixture.state,
        &fixture.jobs,
        crate::network::Update::NotSent { request: prefix },
    );
    assert!(!suffix.validity.as_ref().unwrap().is_current());
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(SUFFIX.to_vec())
    ));
    fixture.no_jobs();
}
