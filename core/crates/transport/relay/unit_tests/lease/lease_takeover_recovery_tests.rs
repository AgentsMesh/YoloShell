use super::*;
use crate::lease::constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::protocol::method;
use crate::relay::session_test_support::{host_status, running_session};
use crate::relay::test_support::{FakeHost, Reply};
use crate::worker::LeaseRequest;

#[path = "lease_takeover_reconcile_tests.rs"]
mod reconcile_tests;

const HOLDER: &str = "takeover-holder";
const INSTANCE: &str = "takeover-instance";
const FIRST_FOREIGN_HOLDER: &str = "foreign-holder-a";
const SECOND_FOREIGN_HOLDER: &str = "foreign-holder-b";
const FOREIGN_INSTANCE: &str = "foreign-instance";
const INITIAL_GENERATION: u64 = 31;
const COMMITTED_GENERATION: u64 = 32;
const SUPERSEDING_GENERATION: u64 = 33;
const LATEST_GENERATION: u64 = 34;
const FINAL_GENERATION: u64 = 35;

fn foreign(holder: &str, generation: u64) -> WriterLease {
    WriterLease {
        holder_id: holder.into(),
        controller_instance_id: Some(FOREIGN_INSTANCE.into()),
        generation,
        expires_at_ms: i64::MAX,
        ttl_ms: DEFAULT_TTL_MS,
        renew_after_ms: DEFAULT_TTL_MS / RENEW_AFTER_DIVISOR,
    }
}

fn controller(host: &FakeHost, terminal_id: Uuid) -> LeaseController {
    let (status, _events) = tokio::sync::mpsc::unbounded_channel();
    LeaseController {
        runtime: RelayRuntime::existing(host.client()),
        target: test_runtime_target(terminal_id),
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
    }
}

fn reconnect(terminal_id: Uuid) -> Vec<Reply> {
    vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
    ]
}

fn takeover_generations(host: &FakeHost) -> Vec<Option<u64>> {
    host.requests()
        .into_iter()
        .filter(|request| request.method == method::TAKEOVER_WRITER)
        .map(|request| request.decode_payload::<LeaseRequest>().unwrap().generation)
        .collect()
}

#[tokio::test]
async fn takeover_failed_before_commit_retries_the_observed_generation() {
    let terminal_id = Uuid::new_v4();
    let occupied = foreign(FIRST_FOREIGN_HOLDER, INITIAL_GENERATION);
    let mut replies = vec![Reply::value(&Some(occupied.clone())), Reply::Disconnect];
    replies.extend(reconnect(terminal_id));
    replies.extend([
        Reply::value(&Some(occupied.clone())),
        Reply::value(&Some(occupied)),
        Reply::Lease(COMMITTED_GENERATION),
    ]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);

    controller.takeover().await.unwrap();
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Takeover);
    assert_eq!(controller.generation(), None);
    controller.maintain().await.unwrap();

    assert_eq!(controller.generation(), Some(COMMITTED_GENERATION));
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Observe);
    assert_eq!(
        takeover_generations(&host),
        [Some(INITIAL_GENERATION), Some(INITIAL_GENERATION)]
    );
}

#[tokio::test]
async fn takeover_committed_reply_loss_recovers_the_original_capability() {
    let terminal_id = Uuid::new_v4();
    let mut replies = vec![
        Reply::value(&Some(foreign(FIRST_FOREIGN_HOLDER, INITIAL_GENERATION))),
        Reply::LeaseThenDisconnect(COMMITTED_GENERATION),
    ];
    replies.extend(reconnect(terminal_id));
    replies.extend([Reply::StoredLease, Reply::Lease(COMMITTED_GENERATION)]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);

    controller.takeover().await.unwrap();

    assert_eq!(controller.generation(), Some(COMMITTED_GENERATION));
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Observe);
    assert_eq!(takeover_generations(&host), [Some(INITIAL_GENERATION)]);
    assert_eq!(
        host.methods().last().map(String::as_str),
        Some(method::ACQUIRE_WRITER)
    );
}

#[tokio::test]
async fn concurrent_supersession_requeries_before_retrying_takeover() {
    let terminal_id = Uuid::new_v4();
    let mut replies = vec![
        Reply::value(&Some(foreign(FIRST_FOREIGN_HOLDER, INITIAL_GENERATION))),
        Reply::LeaseThenDisconnect(COMMITTED_GENERATION),
    ];
    replies.extend(reconnect(terminal_id));
    replies.extend([
        Reply::value(&Some(foreign(
            SECOND_FOREIGN_HOLDER,
            SUPERSEDING_GENERATION,
        ))),
        Reply::value(&Some(foreign(SECOND_FOREIGN_HOLDER, LATEST_GENERATION))),
        Reply::Lease(FINAL_GENERATION),
    ]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);

    controller.takeover().await.unwrap();
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Takeover);
    controller.maintain().await.unwrap();

    assert_eq!(controller.generation(), Some(FINAL_GENERATION));
    assert_eq!(
        takeover_generations(&host),
        [Some(INITIAL_GENERATION), Some(LATEST_GENERATION)]
    );
}
