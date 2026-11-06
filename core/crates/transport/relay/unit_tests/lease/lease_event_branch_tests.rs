use super::*;
use crate::lease::constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::protocol::{EventEnvelope, event};
use tokio::sync::mpsc;

const HOLDER: &str = "holder-alpha";
const CONTROLLER: &str = "controller-alpha";
const OTHER_HOLDER: &str = "holder-beta";
const OTHER_CONTROLLER: &str = "controller-beta";
const GENERATION: u64 = 17;

fn writer(holder: &str, controller: &str) -> WriterLease {
    WriterLease {
        holder_id: holder.into(),
        controller_instance_id: Some(controller.into()),
        generation: GENERATION,
        expires_at_ms: DEFAULT_TTL_MS as i64,
        ttl_ms: DEFAULT_TTL_MS,
        renew_after_ms: DEFAULT_TTL_MS / RENEW_AFTER_DIVISOR,
    }
}

fn event(event_type: &str, lease: &WriterLease) -> EventEnvelope {
    EventEnvelope::new(event_type, None, None, lease).unwrap()
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

#[test]
fn event_identity_comparison_checks_holder_and_controller_after_generation() {
    let (mut controller, mut statuses) = controller();
    controller
        .set_writer_for_test(writer(HOLDER, CONTROLLER))
        .unwrap();
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::ControlAcquired);

    controller
        .observe_event(&event(
            event::WRITER_LEASE_REVOKED,
            &writer(OTHER_HOLDER, CONTROLLER),
        ))
        .unwrap();
    controller
        .observe_event(&event(
            event::WRITER_LEASE_REVOKED,
            &writer(HOLDER, OTHER_CONTROLLER),
        ))
        .unwrap();
    assert_eq!(controller.generation(), Some(GENERATION));
    assert!(statuses.try_recv().is_err());
}

#[test]
fn acquired_and_conflicted_events_do_not_replace_an_existing_writer() {
    let (mut controller, mut statuses) = controller();
    controller
        .set_writer_for_test(writer(HOLDER, CONTROLLER))
        .unwrap();
    statuses.try_recv().unwrap();
    controller
        .observe_event(&event(
            event::WRITER_LEASE_ACQUIRED,
            &writer(OTHER_HOLDER, OTHER_CONTROLLER),
        ))
        .unwrap();
    controller
        .observe_event(&event(
            event::WRITER_LEASE_CONFLICTED,
            &writer(OTHER_HOLDER, OTHER_CONTROLLER),
        ))
        .unwrap();
    controller
        .observe_event(&event(
            event::WRITER_LEASE_ACQUIRED,
            &writer(HOLDER, CONTROLLER),
        ))
        .unwrap();
    assert_eq!(controller.generation(), Some(GENERATION));
    assert!(statuses.try_recv().is_err());
}

#[test]
fn replacement_acquisition_cannot_overwrite_sticky_control_loss() {
    let (mut controller, mut statuses) = controller();
    controller
        .set_writer_for_test(writer(HOLDER, CONTROLLER))
        .unwrap();
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::ControlAcquired);

    controller
        .observe_event(&event(
            event::WRITER_LEASE_REVOKED,
            &writer(HOLDER, CONTROLLER),
        ))
        .unwrap();
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ControlLost { holder_id: None }
    );

    controller
        .observe_event(&event(
            event::WRITER_LEASE_ACQUIRED,
            &writer(OTHER_HOLDER, OTHER_CONTROLLER),
        ))
        .unwrap();
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ControlLost {
            holder_id: Some(OTHER_HOLDER.into())
        }
    );
}

#[test]
fn loss_transition_is_announced_when_the_conflict_holder_is_unchanged() {
    let (mut controller, mut statuses) = controller();
    let holder = Some(OTHER_HOLDER.to_owned());
    controller.set_read_only(holder.clone(), false, false);
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ReadOnly {
            holder_id: holder.clone()
        }
    );

    controller.set_read_only(holder.clone(), true, false);
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ControlLost { holder_id: holder }
    );
}
