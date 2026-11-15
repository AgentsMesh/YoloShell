use super::*;
use crate::custody_guardian::settlement_containment::{
    NAMESPACE_LOSS_GRACEFUL_GRACE, NAMESPACE_LOSS_OBSERVATION_GRACE, NamespaceLossContainment,
    NamespaceLossExactSettlement,
};
use crate::process::{ProcessIdentityObservation, ProcessIdentityRelation};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::ExitStatusExt;
use std::time::Instant;

const SLAVE_OPEN_FLAGS: libc::c_int = libc::O_NOCTTY | libc::O_CLOEXEC | libc::O_NONBLOCK;
const QUEUED_PENDING_OBSERVATIONS: usize = 3;
const UNEXPECTED_REFINALIZATION_DETAIL: &str =
    "finalized namespace-loss seal was recomputed after queued holder materialization";

#[test]
fn queued_holder_materialized_after_finalize_is_contained() {
    let _isolation = serialize_real_pty_containment_tests();
    let (mut fixture, _worker) = PrematureFixture::new(ProcessIdentityRelation::Gone);
    let mut holder = queue_slave_right(&mut fixture);
    let origin = tokio::time::Instant::now();
    let mut containment = prepare_finalized(&mut fixture, origin);
    let exact = wait_for_exact(&fixture.custody, &mut containment, force_time(origin));
    assert!(
        namespace_processes_settled(&fixture.custody, &fixture.runtime, Some(exact))
            .unwrap()
            .is_none()
    );

    holder.materialize();
    assert_eq!(
        holder.identity().observe(),
        ProcessIdentityObservation::Matching
    );
    let _refinalization = crate::process::install_namespace_loss_pre_finalize_hook(|| {
        anyhow::bail!(UNEXPECTED_REFINALIZATION_DETAIL)
    })
    .unwrap();
    let scans_before = containment.holder_discovery_scans_for_test();
    let settled = wait_for_settlement_without_errors(
        &fixture.custody,
        &fixture.runtime,
        &mut containment,
        force_time(origin),
    );
    let discovery_scans = containment
        .holder_discovery_scans_for_test()
        .saturating_sub(scans_before);

    assert_eq!(settled.worker, worker_identity(&fixture.runtime));
    assert_eq!(settled.child, child_identity(&fixture.runtime));
    assert_ne!(discovery_scans, usize::default());
    assert_eq!(holder.wait_for_exit().signal(), Some(libc::SIGKILL));
}

#[test]
fn queued_right_remains_pending_until_its_receiver_closes() {
    let _isolation = serialize_real_pty_containment_tests();
    let (mut fixture, _worker) = PrematureFixture::new(ProcessIdentityRelation::Gone);
    let holder = queue_slave_right(&mut fixture);
    let queued_at = Instant::now();
    let origin = tokio::time::Instant::now();
    let mut containment = prepare_finalized(&mut fixture, origin);
    let force = force_time(origin);
    let scans_before = containment.holder_discovery_scans_for_test();

    for _observation in 0..QUEUED_PENDING_OBSERVATIONS {
        let holder_before = holder.identity().observe();
        let exact = wait_for_exact(&fixture.custody, &mut containment, force);
        let settled = namespace_processes_settled(&fixture.custody, &fixture.runtime, Some(exact))
            .unwrap();
        let holder_after = holder.identity().observe();
        assert!(
            settled.is_none(),
            "queued right settled before explicit closure: holder_before={holder_before:?}; holder_after={holder_after:?}; elapsed={:?}",
            queued_at.elapsed()
        );
        assert_eq!(
            holder_after,
            ProcessIdentityObservation::Matching
        );
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
    let pending_scans = containment
        .holder_discovery_scans_for_test()
        .saturating_sub(scans_before);
    assert!(pending_scans < QUEUED_PENDING_OBSERVATIONS);
    drop(holder);

    let settled = wait_for_settlement_without_errors(
        &fixture.custody,
        &fixture.runtime,
        &mut containment,
        force,
    );
    assert_eq!(settled.worker, worker_identity(&fixture.runtime));
    assert_eq!(settled.child, child_identity(&fixture.runtime));
}

fn queue_slave_right(fixture: &mut PrematureFixture) -> queued_holder::QueuedPtyHolder {
    let queued = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(SLAVE_OPEN_FLAGS)
        .open(fixture.custody.pty_slave_path_for_test())
        .unwrap();
    let mut holder = queued_holder::QueuedPtyHolder::spawn();
    holder.queue(&queued);
    drop(queued);
    drop(fixture.take_inherited_slave());
    holder
}

fn prepare_finalized(
    fixture: &mut PrematureFixture,
    origin: tokio::time::Instant,
) -> NamespaceLossContainment {
    let mut containment = NamespaceLossContainment::new(&fixture.runtime, origin);
    let deadline = Instant::now() + TEST_TIMEOUT;
    reconcile_before(&fixture.custody, &mut containment, origin, deadline);
    assert_eq!(fixture.terminate_child().signal(), Some(libc::SIGKILL));
    reconcile_before(
        &fixture.custody,
        &mut containment,
        origin + NAMESPACE_LOSS_OBSERVATION_GRACE,
        deadline,
    );
    containment
}

fn force_time(origin: tokio::time::Instant) -> tokio::time::Instant {
    origin + NAMESPACE_LOSS_OBSERVATION_GRACE + NAMESPACE_LOSS_GRACEFUL_GRACE
}

fn wait_for_exact(
    custody: &MacosSecondaryCustody,
    containment: &mut NamespaceLossContainment,
    now: tokio::time::Instant,
) -> NamespaceLossExactSettlement {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if let Some(exact) = reconcile_before(custody, containment, now, deadline) {
            return exact;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_for_settlement_without_errors(
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    containment: &mut NamespaceLossContainment,
    now: tokio::time::Instant,
) -> NamespaceRuntimeSettled {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let exact = reconcile_before(custody, containment, now, deadline);
        if let Some(settled) = namespace_processes_settled(custody, runtime, exact).unwrap() {
            return settled;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}
