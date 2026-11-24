use super::*;
use crate::relay::constants::LEASE_CLEANUP_POLL_ATTEMPTS;
use crate::relay::test_support::{FakeHost, Reply};

const HOLDER: &str = "cleanup-holder";
const INSTANCE: &str = "cleanup-instance";
const OTHER_HOLDER: &str = "other-holder";
const OTHER_INSTANCE: &str = "other-instance";
const GENERATION: u64 = 17;
const NEXT_GENERATION: u64 = GENERATION + 1;
const STALE_EXPIRY_MS: i64 = 1;

fn public_lease() -> WriterLease {
    WriterLease {
        holder_id: HOLDER.into(),
        controller_instance_id: Some(INSTANCE.into()),
        generation: GENERATION,
        expires_at_ms: STALE_EXPIRY_MS,
        ttl_ms: crate::lease::constants::DEFAULT_TTL_MS,
        renew_after_ms: crate::lease::constants::DEFAULT_TTL_MS
            / crate::lease::constants::RENEW_AFTER_DIVISOR,
    }
}

fn uncertain_controller(host: &FakeHost) -> LeaseController {
    let (status, _events) = tokio::sync::mpsc::unbounded_channel();
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
        ownership_uncertain: true,
        control_lost: false,
        lifecycle: SessionLifecycle::Active,
    }
}

#[tokio::test]
async fn uncertain_cleanup_releases_a_renewed_matching_incarnation() {
    let public = public_lease();
    let host = FakeHost::start(vec![
        Reply::value(&Some(public)),
        Reply::Lease(GENERATION),
        Reply::value(&()),
    ])
    .await;
    uncertain_controller(&host).release().await;
    assert_eq!(
        host.methods(),
        [
            crate::protocol::method::GET_WRITER_LEASE,
            crate::protocol::method::ACQUIRE_WRITER,
            crate::protocol::method::RELEASE_WRITER,
        ]
    );
}

#[tokio::test]
async fn uncertain_cleanup_stops_for_absent_foreign_and_failed_lookups() {
    let absent = FakeHost::start(vec![
        Reply::value(&None::<WriterLease>),
        Reply::value(&None::<WriterLease>),
        Reply::value(&None::<WriterLease>),
        Reply::value(&None::<WriterLease>),
    ])
    .await;
    uncertain_controller(&absent).release().await;

    let foreign = WriterLease {
        holder_id: "foreign-holder".into(),
        controller_instance_id: None,
        generation: 1,
        expires_at_ms: i64::MAX,
        ttl_ms: 1,
        renew_after_ms: 1,
    };
    let occupied = FakeHost::start(vec![Reply::value(&Some(foreign.clone()))]).await;
    uncertain_controller(&occupied).release().await;
    let failed = FakeHost::start(vec![Reply::Disconnect]).await;
    uncertain_controller(&failed).release().await;

    assert_eq!(absent.methods().len(), LEASE_CLEANUP_POLL_ATTEMPTS);
    assert_eq!(
        occupied.methods(),
        [crate::protocol::method::GET_WRITER_LEASE]
    );
    assert_eq!(
        failed.methods(),
        [crate::protocol::method::GET_WRITER_LEASE]
    );
}

#[tokio::test]
async fn uncertain_cleanup_rejects_failed_and_mismatched_recovery() {
    let failed = FakeHost::start(vec![
        Reply::value(&Some(public_lease())),
        Reply::error(crate::protocol::error_code::INTERNAL),
    ])
    .await;
    uncertain_controller(&failed).release().await;

    let mismatched = FakeHost::start(vec![
        Reply::value(&Some(public_lease())),
        Reply::Lease(NEXT_GENERATION),
    ])
    .await;
    uncertain_controller(&mismatched).release().await;
    for host in [&failed, &mismatched] {
        assert_eq!(
            host.methods(),
            [
                crate::protocol::method::GET_WRITER_LEASE,
                crate::protocol::method::ACQUIRE_WRITER,
            ]
        );
    }
}

#[test]
fn lease_incarnation_comparison_checks_each_identity_dimension() {
    let current = public_lease();
    for changed in [
        WriterLease {
            holder_id: OTHER_HOLDER.into(),
            ..current.clone()
        },
        WriterLease {
            controller_instance_id: Some(OTHER_INSTANCE.into()),
            ..current.clone()
        },
        WriterLease {
            generation: NEXT_GENERATION,
            ..current.clone()
        },
    ] {
        assert!(!release::same_lease_incarnation(&current, &changed));
    }
    assert!(release::same_lease_incarnation(&current, &current));
}
