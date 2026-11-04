use super::*;
use crate::lease::constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::protocol::error_code::INTERNAL as INTERNAL_CODE;
use crate::protocol::method;
use crate::relay::constants::error_code;
use crate::relay::test_support::{FakeHost, Reply};
use tokio::sync::mpsc;

#[path = "lease_lifecycle_recovery_tests.rs"]
mod recovery_tests;

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

fn make_controller(
    host: &FakeHost,
    terminal_id: Uuid,
    current: Option<WriterLease>,
    retry_enabled: bool,
) -> (LeaseController, mpsc::UnboundedReceiver<RelayStatus>) {
    let (status, events) = mpsc::unbounded_channel();
    let writer_capability = WriterCapability::generate();
    let current = current.map(|lease| WriterLeaseGrant {
        lease,
        writer_capability: writer_capability.clone(),
    });
    let retry_mode = if retry_enabled {
        LeaseAcquireMode::Observe
    } else {
        LeaseAcquireMode::Disabled
    };
    (
        LeaseController {
            runtime: RelayRuntime::existing(host.client()),
            target: test_runtime_target(terminal_id),
            identity: LeaseIdentity {
                holder_id: HOLDER.into(),
                controller_instance_id: INSTANCE.into(),
            },
            writer_capability,
            lease: current,
            conflict_holder: None,
            retry_mode,
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

fn remote(code: &str) -> ClientError {
    ClientError::Remote {
        code: code.into(),
        message: code.into(),
        retryable: false,
        details: None,
    }
}

#[tokio::test]
async fn maintenance_renews_and_surfaces_non_lease_errors() {
    let renewed = lease(HOLDER, INSTANCE, 4);
    let host = FakeHost::start(vec![Reply::Lease(4)]).await;
    let (mut controller, mut statuses) =
        make_controller(&host, Uuid::new_v4(), Some(renewed.clone()), true);
    controller.maintain().await.unwrap();
    assert_eq!(controller.generation(), Some(4));
    assert!(statuses.try_recv().is_err());

    let host = FakeHost::start(vec![Reply::error(INTERNAL_CODE)]).await;
    let (mut controller, _) = make_controller(&host, Uuid::new_v4(), Some(renewed), true);
    assert!(matches!(
        controller.maintain().await,
        Err(RelayError::Client(_))
    ));
}

#[tokio::test]
async fn ownership_failure_reconciles_to_the_authoritative_holder() {
    let other = lease(OTHER_HOLDER, OTHER_INSTANCE, 9);
    let host = FakeHost::start(vec![
        Reply::error(error_code::STALE_GENERATION),
        Reply::value(&Some(other.clone())),
    ])
    .await;
    let (mut controller, mut statuses) =
        make_controller(
            &host,
            Uuid::new_v4(),
            Some(lease(HOLDER, INSTANCE, 4)),
            true,
        );
    controller.maintain().await.unwrap();
    assert_eq!(controller.generation(), None);
    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::ControlLost {
            holder_id: Some(OTHER_HOLDER.into())
        }
    );
    assert_eq!(
        host.methods(),
        vec![method::RENEW_WRITER, method::GET_WRITER_LEASE]
    );
}

#[tokio::test]
async fn maintenance_handles_read_only_retry_and_disabled_retry() {
    let host = FakeHost::start(vec![Reply::Lease(6)]).await;
    let (mut retrying, _) = make_controller(&host, Uuid::new_v4(), None, true);
    retrying.maintain().await.unwrap();
    assert_eq!(retrying.generation(), Some(6));

    let host = FakeHost::start(Vec::new()).await;
    let (mut disabled, _) = make_controller(&host, Uuid::new_v4(), None, false);
    disabled.maintain().await.unwrap();
    assert_eq!(disabled.generation(), None);
    assert!(host.methods().is_empty());
}

#[tokio::test]
async fn mutation_classification_and_release_are_fenced() {
    let current = lease(HOLDER, INSTANCE, 3);
    let host = FakeHost::start(vec![
        Reply::value(&Some(current.clone())),
        Reply::Lease(3),
        Reply::value(&()),
    ])
    .await;
    let (mut controller, _) =
        make_controller(&host, Uuid::new_v4(), Some(current), true);
    assert!(
        !controller
            .mutation_rejected(&remote(INTERNAL_CODE))
            .await
            .unwrap()
    );
    assert!(
        controller
            .mutation_rejected(&remote(error_code::NOT_HOLDER))
            .await
            .unwrap()
    );
    controller.release().await;
    controller.release().await;
    assert_eq!(
        host.methods(),
        vec![
            method::GET_WRITER_LEASE,
            method::RENEW_WRITER,
            method::RELEASE_WRITER
        ]
    );
}
