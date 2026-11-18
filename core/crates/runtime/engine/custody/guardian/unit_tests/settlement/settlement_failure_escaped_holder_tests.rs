use super::*;
use crate::custody_guardian::settlement_containment::{
    NAMESPACE_LOSS_GRACEFUL_GRACE, NAMESPACE_LOSS_OBSERVATION_GRACE, NamespaceLossContainment,
    NamespaceLossExactSettlement,
};
use crate::process::{ProcessIdentityObservation, ProcessIdentityRelation};
use std::time::Instant;

#[path = "settlement_failure_reopen_watcher.rs"]
mod watcher;

const HOLDER_NOT_GONE_DETAIL: &str = "reopen watcher ran before the exact holder was gone";
const HOLDER_DISCOVERY_TIMEOUT_DETAIL: &str =
    "escaped PTY holder discovery did not converge before its deadline";

#[test]
fn preescaped_pty_holder_requires_positive_containment_and_kernel_eof() {
    let _isolation = serialize_real_pty_containment_tests();
    let (mut fixture, _worker, mut holder) =
        PrematureFixture::with_escaped_holder(ProcessIdentityRelation::Gone);
    drop(fixture.take_inherited_slave());
    let holder_identity = holder.identity();
    assert_eq!(
        holder_identity.observe(),
        ProcessIdentityObservation::Matching
    );
    let origin = tokio::time::Instant::now();
    let mut containment = NamespaceLossContainment::new(&fixture.runtime, origin);
    assert!(reconcile_with_retry(&fixture, &mut containment, origin).is_none());
    assert!(containment.tracks_holder(holder_identity));
    let watcher = watcher::ReopenWatcher::spawn(holder.pty_path());
    let _watcher_hook = crate::process::install_namespace_loss_pre_finalize_hook(move || {
        anyhow::ensure!(
            holder_identity.observe() == ProcessIdentityObservation::Gone,
            HOLDER_NOT_GONE_DETAIL
        );
        watcher.prove_reopen_rejected()
    })
    .unwrap();
    assert_eq!(fixture.terminate_child().signal(), Some(libc::SIGKILL));
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    assert!(reconcile_with_retry(&fixture, &mut containment, graceful).is_none());
    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;
    let settled = wait_for_scanned_holder_settlement(&fixture, &mut containment, force);
    assert_eq!(settled.worker, worker_identity(&fixture.runtime));
    assert_eq!(settled.child, child_identity(&fixture.runtime));
    assert!(!containment.tracks_holder(holder_identity));
    holder.wait_for_exit();
}

fn wait_for_scanned_holder_settlement(
    fixture: &PrematureFixture,
    containment: &mut NamespaceLossContainment,
    now: tokio::time::Instant,
) -> NamespaceRuntimeSettled {
    let deadline = Instant::now() + fixture::TEST_TIMEOUT;
    loop {
        let exact = reconcile_with_retry(fixture, containment, now);
        if let Some(exact) = exact {
            match namespace_processes_settled(&fixture.custody, &fixture.runtime, Some(exact)) {
                Ok(Some(settled)) => return settled,
                Ok(None) | Err(_) => {}
            }
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(fixture::TEST_POLL_INTERVAL);
    }
}

fn reconcile_with_retry(
    fixture: &PrematureFixture,
    containment: &mut NamespaceLossContainment,
    now: tokio::time::Instant,
) -> Option<NamespaceLossExactSettlement> {
    let deadline = Instant::now() + fixture::TEST_TIMEOUT;
    loop {
        match containment.reconcile_custody(&fixture.custody, now) {
            Ok(exact) => return exact,
            Err(error) => {
                assert!(
                    Instant::now() < deadline,
                    "{HOLDER_DISCOVERY_TIMEOUT_DETAIL}: {error:#}"
                );
                std::thread::sleep(fixture::TEST_POLL_INTERVAL);
            }
        }
    }
}
