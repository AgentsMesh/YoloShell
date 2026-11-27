use super::*;
use crate::lease::constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::protocol::error_code::INTERNAL as INTERNAL_CODE;
use crate::relay::constants::error_code;
use crate::relay::test_support::{FakeHost, Reply};
use tokio::sync::mpsc;

const HOLDER: &str = "holder-owner";
const INSTANCE: &str = "relay-instance";
const OTHER_HOLDER: &str = "other-holder";
const OTHER_INSTANCE: &str = "other-instance";

fn lease(holder: &str, instance: &str, generation: u64) -> WriterLease {
    WriterLease {
        holder_id: holder.into(),
        controller_instance_id: Some(instance.into()),
        generation,
        expires_at_ms: (DEFAULT_TTL_MS * 2) as i64,
        ttl_ms: DEFAULT_TTL_MS,
        renew_after_ms: DEFAULT_TTL_MS / RENEW_AFTER_DIVISOR,
    }
}

fn make_controller(host: &FakeHost) -> (LeaseController, mpsc::UnboundedReceiver<RelayStatus>) {
    let (status, events) = mpsc::unbounded_channel();
    (
        LeaseController {
            runtime: RelayRuntime::existing(host.client()),
            target: test_runtime_target(Uuid::new_v4()),
            identity: LeaseIdentity {
                holder_id: HOLDER.into(),
                controller_instance_id: INSTANCE.into(),
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

#[tokio::test]
async fn acquire_conflict_reports_read_only_and_other_errors_propagate() {
    let other = lease(OTHER_HOLDER, OTHER_INSTANCE, 3);
    let host = FakeHost::start(vec![
        Reply::error(error_code::HELD_BY_OTHER),
        Reply::value(&Some(other)),
    ])
    .await;
    let (mut controller, mut statuses) = make_controller(&host);
    controller.acquire().await.unwrap();
    assert_eq!(controller.generation(), None);
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ReadOnly {
            holder_id: Some(OTHER_HOLDER.into())
        }
    );

    let host = FakeHost::start(vec![Reply::error(INTERNAL_CODE)]).await;
    let (mut controller, _) = make_controller(&host);
    assert!(matches!(
        controller.acquire().await,
        Err(RelayError::Client(_))
    ));
}

#[tokio::test]
async fn takeover_acquires_when_free_or_when_the_same_holder_restarts() {
    let host = FakeHost::start(vec![
        Reply::value(&Option::<WriterLease>::None),
        Reply::Lease(4),
    ])
    .await;
    let (mut controller, _) = make_controller(&host);
    controller.takeover().await.unwrap();
    assert_eq!(controller.generation(), Some(4));

    let prior = lease(HOLDER, "prior-instance", 4);
    let host = FakeHost::start(vec![Reply::value(&Some(prior)), Reply::Lease(5)]).await;
    let (mut controller, _) = make_controller(&host);
    controller.takeover().await.unwrap();
    assert_eq!(controller.generation(), Some(5));
    assert_eq!(
        host.methods(),
        vec![
            crate::protocol::method::GET_WRITER_LEASE,
            crate::protocol::method::TAKEOVER_WRITER,
        ]
    );
}

#[tokio::test]
async fn takeover_of_another_holder_uses_the_observed_generation() {
    let prior = lease(OTHER_HOLDER, OTHER_INSTANCE, 8);
    let host = FakeHost::start(vec![Reply::value(&Some(prior)), Reply::Lease(9)]).await;
    let (mut controller, _) = make_controller(&host);
    controller.takeover().await.unwrap();
    assert_eq!(controller.generation(), Some(9));
    assert_eq!(
        host.methods(),
        vec![
            crate::protocol::method::GET_WRITER_LEASE,
            crate::protocol::method::TAKEOVER_WRITER,
        ]
    );
}

#[tokio::test]
async fn takeover_contention_is_bounded_and_non_race_errors_stop_immediately() {
    let mut replies = Vec::new();
    for generation in 1..=super::super::constants::TAKEOVER_MAX_ATTEMPTS {
        replies.push(Reply::value(&Some(lease(
            OTHER_HOLDER,
            OTHER_INSTANCE,
            generation as u64,
        ))));
        replies.push(Reply::error(error_code::STALE_GENERATION));
    }
    let host = FakeHost::start(replies).await;
    let (mut controller, _) = make_controller(&host);
    assert!(matches!(
        controller.takeover().await,
        Err(RelayError::TakeoverContended)
    ));

    let host = FakeHost::start(vec![
        Reply::value(&Some(lease(OTHER_HOLDER, OTHER_INSTANCE, 1))),
        Reply::error(INTERNAL_CODE),
    ])
    .await;
    let (mut controller, _) = make_controller(&host);
    assert!(matches!(
        controller.takeover().await,
        Err(RelayError::Client(_))
    ));
}
