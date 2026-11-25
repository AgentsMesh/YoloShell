use super::*;
use crate::lease::constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR, code};
use crate::lease::{LeaseEventPayload, LeaseReason};
use crate::protocol::{EventEnvelope, event, method};
use crate::relay::test_support::{FakeHost, Reply};
use tokio::sync::mpsc;

const HOLDER: &str = "ending-holder";
const INSTANCE: &str = "ending-controller";
const OTHER_HOLDER: &str = "ending-other-holder";
const OTHER_INSTANCE: &str = "ending-other-controller";
const GENERATION: u64 = 23;

fn writer(holder: &str, instance: &str) -> WriterLease {
    WriterLease {
        holder_id: holder.into(),
        controller_instance_id: Some(instance.into()),
        generation: GENERATION,
        expires_at_ms: DEFAULT_TTL_MS as i64,
        ttl_ms: DEFAULT_TTL_MS,
        renew_after_ms: DEFAULT_TTL_MS / RENEW_AFTER_DIVISOR,
    }
}

fn controller(
    host: &FakeHost,
    held: bool,
) -> (LeaseController, mpsc::UnboundedReceiver<RelayStatus>) {
    let (status, events) = mpsc::unbounded_channel();
    let writer_capability = WriterCapability::generate();
    let lease = held.then(|| WriterLeaseGrant {
        lease: writer(HOLDER, INSTANCE),
        writer_capability: writer_capability.clone(),
    });
    (
        LeaseController {
            runtime: RelayRuntime::existing(host.client()),
            target: test_runtime_target(Uuid::new_v4()),
            identity: LeaseIdentity {
                holder_id: HOLDER.into(),
                controller_instance_id: INSTANCE.into(),
            },
            writer_capability,
            lease,
            conflict_holder: Some(OTHER_HOLDER.into()),
            retry_mode: LeaseAcquireMode::Takeover,
            prepared_generation: Some(GENERATION),
            status_announced: false,
            next_maintenance: Instant::now(),
            status: status.into(),
            ownership_uncertain: true,
            control_lost: true,
            lifecycle: SessionLifecycle::Active,
        },
        events,
    )
}

fn internal_error() -> ClientError {
    ClientError::Remote {
        code: crate::protocol::error_code::INTERNAL.into(),
        message: crate::protocol::error_code::INTERNAL.into(),
        retryable: false,
        details: None,
    }
}

fn lease_event(event_type: &str, reason: Option<LeaseReason>) -> EventEnvelope {
    let payload = LeaseEventPayload::new(writer(HOLDER, INSTANCE), reason);
    EventEnvelope::new(event_type, None, None, &payload).unwrap()
}

fn assert_terminal(controller: &LeaseController, expected: SessionLifecycle) {
    assert_eq!(controller.lifecycle, expected);
    assert!(controller.is_terminal());
    assert_eq!(controller.generation(), None);
    assert_eq!(controller.mutation_credentials(), None);
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Disabled);
    assert_eq!(controller.prepared_generation, None);
    assert_eq!(controller.conflict_holder, None);
    assert!(!controller.ownership_uncertain);
    assert!(!controller.control_lost);
}

#[tokio::test]
async fn acquire_terminal_error_announces_once_and_fences_all_lease_io() {
    let host = FakeHost::start(vec![Reply::error(code::SESSION_NOT_RUNNING)]).await;
    let (mut controller, mut statuses) = controller(&host, false);
    controller.acquire().await.unwrap();
    assert_terminal(&controller, SessionLifecycle::Ending);
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionEnding);

    controller.acquire().await.unwrap();
    controller.takeover().await.unwrap();
    controller.maintain().await.unwrap();
    controller.recover_transport().await.unwrap();
    controller.reconcile(true).await.unwrap();
    controller.retain_control_intent();
    controller.release().await;
    assert!(statuses.try_recv().is_err());
    assert_eq!(host.methods(), [method::ACQUIRE_WRITER]);
}

#[tokio::test]
async fn renew_and_takeover_terminal_responses_enter_the_same_ending_fence() {
    let host = FakeHost::start(vec![Reply::error(code::SESSION_NOT_RUNNING)]).await;
    let (mut renewing, mut statuses) = controller(&host, true);
    renewing.maintain().await.unwrap();
    assert_terminal(&renewing, SessionLifecycle::Ending);
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionEnding);
    assert_eq!(host.methods(), [method::RENEW_WRITER]);

    let foreign = writer(OTHER_HOLDER, OTHER_INSTANCE);
    let host = FakeHost::start(vec![
        Reply::value(&Some(foreign)),
        Reply::error(code::SESSION_NOT_RUNNING),
    ])
    .await;
    let (mut taking, mut statuses) = controller(&host, false);
    taking.takeover().await.unwrap();
    assert_terminal(&taking, SessionLifecycle::Ending);
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionEnding);
    assert_eq!(host.methods(), [method::GET_WRITER_LEASE, method::TAKEOVER_WRITER]);
}

#[tokio::test]
async fn session_ended_revocation_ends_writers_and_observers_exactly_once() {
    for held in [true, false] {
        let host = FakeHost::start(Vec::new()).await;
        let (mut controller, mut statuses) = controller(&host, held);
        controller
            .observe_event(&lease_event(
                event::WRITER_LEASE_REVOKED,
                Some(LeaseReason::SessionEnded),
            ))
            .unwrap();
        assert_terminal(&controller, SessionLifecycle::Ending);
        assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionEnding);

        controller
            .observe_event(&lease_event(event::WRITER_LEASE_ACQUIRED, None))
            .unwrap();
        controller
            .observe_event(&lease_event(
                event::WRITER_LEASE_REVOKED,
                Some(LeaseReason::SessionEnded),
            ))
            .unwrap();
        let malformed = EventEnvelope::new(
            event::WRITER_LEASE_REVOKED,
            None,
            None,
            &serde_json::json!({}),
        )
        .unwrap();
        assert!(controller.observe_event(&malformed).is_err());
        assert!(
            !controller
                .mutation_rejected(&internal_error())
                .await
                .unwrap()
        );
        controller.maintain().await.unwrap();
        controller.release().await;
        assert!(statuses.try_recv().is_err());
        assert!(host.methods().is_empty());
    }
}
