use super::*;
use crate::lease::{
    LeaseEventPayload, LeaseReason,
    constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR},
};
use crate::protocol::{EventEnvelope, event};
use tokio::sync::mpsc;

const HOLDER: &str = "holder-alpha";
const CONTROLLER: &str = "controller-alpha";
const OTHER_HOLDER: &str = "holder-beta";
fn writer(holder: &str, controller: &str, generation: u64) -> WriterLease {
    WriterLease {
        holder_id: holder.into(),
        controller_instance_id: Some(controller.into()),
        generation,
        expires_at_ms: DEFAULT_TTL_MS as i64,
        ttl_ms: DEFAULT_TTL_MS,
        renew_after_ms: DEFAULT_TTL_MS / RENEW_AFTER_DIVISOR,
    }
}
fn controller() -> (LeaseController, mpsc::UnboundedReceiver<RelayStatus>) {
    let (status, events) = mpsc::unbounded_channel();
    (
        LeaseController {
            runtime: RelayRuntime::existing(crate::client::Client::new("/tmp/not-connected")),
            target: test_runtime_target(Uuid::new_v4()),
            identity: LeaseIdentity {
                holder_id: HOLDER.into(),
                controller_instance_id: CONTROLLER.into(),
            },
            writer_capability: WriterCapability::generate(),
            lease: None,
            conflict_holder: None,
            retry_mode: LeaseAcquireMode::Observe,
            prepared_generation: None,
            status_announced: false,
            next_maintenance: Instant::now(),
            status: status.into(),
            ownership_uncertain: false,
            control_lost: false,
            lifecycle: SessionLifecycle::Active,
        },
        events,
    )
}
fn lease_event(
    event_type: &str,
    lease: &WriterLease,
    reason: Option<LeaseReason>,
) -> EventEnvelope {
    let payload = LeaseEventPayload::new(lease.clone(), reason);
    EventEnvelope::new(event_type, None, None, &payload).unwrap()
}

#[test]
fn revoked_current_generation_becomes_read_only_without_reacquiring() {
    let (mut controller, mut statuses) = controller();
    controller
        .set_writer_for_test(writer(HOLDER, CONTROLLER, 7))
        .unwrap();
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::ControlAcquired);
    let revoked = lease_event(
        event::WRITER_LEASE_REVOKED,
        controller.lease.as_ref().unwrap(),
        Some(LeaseReason::ControllerReplaced),
    );
    controller.observe_event(&revoked).unwrap();
    assert_eq!(controller.generation(), None);
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Disabled);
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ControlLost { holder_id: None }
    );
}

#[test]
fn unrelated_revocation_cannot_fence_the_current_writer() {
    let (mut controller, _statuses) = controller();
    controller
        .set_writer_for_test(writer(HOLDER, CONTROLLER, 7))
        .unwrap();
    let event = lease_event(
        event::WRITER_LEASE_REVOKED,
        &writer(OTHER_HOLDER, "other-controller", 8),
        Some(LeaseReason::TakenOver),
    );
    controller.observe_event(&event).unwrap();
    assert_eq!(controller.generation(), Some(7));
}

#[test]
fn lease_events_reject_unknown_fields_and_reasons() {
    let (mut controller, _statuses) = controller();
    for payload in [
        serde_json::json!({
            (crate::protocol::field::HOLDER_ID): HOLDER,
            (crate::protocol::field::CONTROLLER_INSTANCE_ID): CONTROLLER,
            (crate::protocol::field::GENERATION): 1,
            (crate::protocol::field::EXPIRES_AT_MS): 2,
            (crate::protocol::field::TTL_MS): 3,
            (crate::protocol::field::RENEW_AFTER_MS): 1,
            (crate::protocol::field::LEASE_GENERATION): 1
        }),
        serde_json::json!({
            (crate::protocol::field::HOLDER_ID): HOLDER,
            (crate::protocol::field::CONTROLLER_INSTANCE_ID): CONTROLLER,
            (crate::protocol::field::GENERATION): 1,
            (crate::protocol::field::EXPIRES_AT_MS): 2,
            (crate::protocol::field::TTL_MS): 3,
            (crate::protocol::field::RENEW_AFTER_MS): 1,
            (crate::protocol::field::REASON): "unknown_reason"
        }),
    ] {
        let event = EventEnvelope::new(event::WRITER_LEASE_REVOKED, None, None, &payload).unwrap();
        assert!(controller.observe_event(&event).is_err());
    }
}

#[test]
fn canonical_lease_validation_rejects_zeroes_and_wrong_identity() {
    let (mut controller, _statuses) = controller();
    assert!(matches!(
        controller.set_writer_for_test(writer(OTHER_HOLDER, CONTROLLER, 1)),
        Err(RelayError::InvalidLease(IDENTITY_MISMATCH))
    ));
    assert!(matches!(
        controller.set_writer_for_test(writer(HOLDER, CONTROLLER, 0)),
        Err(RelayError::InvalidLease(INVALID_GENERATION))
    ));
    let mut invalid = writer(HOLDER, CONTROLLER, 1);
    invalid.renew_after_ms = 0;
    assert!(matches!(
        controller.set_writer_for_test(invalid),
        Err(RelayError::InvalidLease(INVALID_RENEWAL))
    ));
}

#[test]
fn expired_writer_retries_while_takeover_fences_reacquisition() {
    let (mut controller, mut statuses) = controller();
    controller
        .set_writer_for_test(writer(HOLDER, CONTROLLER, 11))
        .unwrap();
    statuses.try_recv().unwrap();
    let expired = lease_event(
        event::WRITER_LEASE_EXPIRED,
        controller.lease.as_ref().unwrap(),
        Some(LeaseReason::Expired),
    );
    controller.observe_event(&expired).unwrap();
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Observe);
    assert!(matches!(
        statuses.try_recv(),
        Ok(RelayStatus::ControlLost { .. })
    ));

    controller
        .set_writer_for_test(writer(HOLDER, CONTROLLER, 12))
        .unwrap();
    statuses.try_recv().unwrap();
    let takeover = lease_event(
        event::WRITER_LEASE_REVOKED,
        controller.lease.as_ref().unwrap(),
        Some(LeaseReason::TakenOver),
    );
    controller.observe_event(&takeover).unwrap();
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Disabled);
}

#[test]
fn acquired_event_updates_read_only_holder_and_irrelevant_events_are_ignored() {
    let (mut controller, mut statuses) = controller();
    let acquired = lease_event(
        event::WRITER_LEASE_ACQUIRED,
        &writer(OTHER_HOLDER, "other-controller", 21),
        None,
    );
    controller.observe_event(&acquired).unwrap();
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ReadOnly {
            holder_id: Some(OTHER_HOLDER.into())
        }
    );
    controller.observe_event(&acquired).unwrap();
    assert!(statuses.try_recv().is_err());

    let ignored = EventEnvelope::new(
        "unrelated",
        None,
        None,
        &serde_json::json!({
            (crate::protocol::field::LEASE_GENERATION): 99
        }),
    )
    .unwrap();
    controller.observe_event(&ignored).unwrap();
}
