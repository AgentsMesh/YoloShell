use super::*;
use crate::lease::constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::protocol::error_code::INTERNAL as INTERNAL_CODE;
use crate::relay::constants::error_code;
use crate::relay::session_test_support::{host_status, running_session};
use crate::relay::test_support::{FakeHost, Reply};

const HOLDER: &str = "boundary-holder";
const INSTANCE: &str = "boundary-controller";
const OTHER_HOLDER: &str = "boundary-other-holder";
const OTHER_INSTANCE: &str = "boundary-other-controller";
const INITIAL_GENERATION: u64 = 71;
const RECOVERED_GENERATION: u64 = 72;

fn lease(holder: &str, instance: &str, generation: u64) -> WriterLease {
    WriterLease {
        holder_id: holder.to_owned(),
        controller_instance_id: Some(instance.to_owned()),
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
            holder_id: HOLDER.to_owned(),
            controller_instance_id: INSTANCE.to_owned(),
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

fn reconnect_replies(terminal_id: Uuid) -> Vec<Reply> {
    vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
    ]
}

fn remote(code: &str) -> ClientError {
    ClientError::Remote {
        code: code.to_owned(),
        message: code.to_owned(),
        retryable: false,
        details: None,
    }
}

#[tokio::test]
async fn disabled_retry_is_a_network_free_noop() {
    let host = FakeHost::start(Vec::new()).await;
    let mut controller = controller(&host, Uuid::new_v4());
    controller.retry_mode = LeaseAcquireMode::Disabled;

    controller.retry_acquisition().await.unwrap();

    assert!(host.methods().is_empty());
}

#[tokio::test]
async fn uncertain_acquire_retries_when_reconnect_finds_no_public_lease() {
    let terminal_id = Uuid::new_v4();
    let mut replies = vec![Reply::Disconnect];
    replies.extend(reconnect_replies(terminal_id));
    replies.extend([
        Reply::value(&Option::<WriterLease>::None),
        Reply::Lease(RECOVERED_GENERATION),
    ]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);

    controller.acquire().await.unwrap();

    assert_eq!(controller.generation(), Some(RECOVERED_GENERATION));
}

#[tokio::test]
async fn ownership_rejection_resolves_the_current_holder() {
    let public = lease(OTHER_HOLDER, OTHER_INSTANCE, INITIAL_GENERATION);
    let host = FakeHost::start(vec![Reply::value(&Some(public))]).await;
    let mut controller = controller(&host, Uuid::new_v4());

    controller
        .finish_acquire(Err(remote(error_code::NOT_HOLDER)), false)
        .await
        .unwrap();

    assert_eq!(controller.conflict_holder.as_deref(), Some(OTHER_HOLDER));
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Disabled);
}

#[tokio::test]
async fn reconciliation_renews_a_matching_public_lease() {
    let terminal_id = Uuid::new_v4();
    let current = lease(HOLDER, INSTANCE, INITIAL_GENERATION);
    let mut replies = reconnect_replies(terminal_id);
    replies.extend([
        Reply::value(&Some(current.clone())),
        Reply::Lease(INITIAL_GENERATION),
    ]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);
    controller.set_writer_for_test(current).unwrap();

    controller.recover_transport().await.unwrap();

    assert_eq!(controller.generation(), Some(INITIAL_GENERATION));
}

#[tokio::test]
async fn reconciliation_classifies_matching_renewal_failures() {
    assert_matching_renewal_failure(error_code::NOT_HOLDER, false).await;
    assert_matching_renewal_failure(INTERNAL_CODE, true).await;
}

async fn assert_matching_renewal_failure(code: &str, propagates: bool) {
    let terminal_id = Uuid::new_v4();
    let current = lease(HOLDER, INSTANCE, INITIAL_GENERATION);
    let mut replies = reconnect_replies(terminal_id);
    replies.extend([
        Reply::value(&Some(current.clone())),
        Reply::error(code),
    ]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);
    controller.set_writer_for_test(current).unwrap();

    let result = controller.recover_transport().await;

    assert_eq!(result.is_err(), propagates);
    assert_eq!(controller.generation().is_none(), !propagates);
}

#[tokio::test]
async fn renewal_transport_loss_reconnects_then_verifies_by_acquire() {
    let terminal_id = Uuid::new_v4();
    let current = lease(HOLDER, INSTANCE, INITIAL_GENERATION);
    let mut replies = reconnect_replies(terminal_id);
    replies.extend([Reply::value(&Some(current.clone())), Reply::Disconnect]);
    replies.extend(reconnect_replies(terminal_id));
    replies.push(Reply::Lease(INITIAL_GENERATION));
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);
    controller.set_writer_for_test(current).unwrap();

    controller.recover_transport().await.unwrap();

    assert_eq!(controller.generation(), Some(INITIAL_GENERATION));
}
