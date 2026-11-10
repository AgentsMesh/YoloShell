use super::*;
use crate::process::{ProcessIdentity, ProcessIdentityObservation};

#[path = "../../test_support/unit/secondary_custody/secondary_custody_non_holder_tree_test.rs"]
mod tree_helper;

const PREMATURE_RELEASE_DETAIL: &str = "process capability writer released before the root exited";
const PREMATURE_RELEASE_MISCLASSIFIED_DETAIL: &str =
    "premature writer EOF was accepted as settled Host custody";
const PROCESS_QUERY_FAILURE: libc::pid_t = -1;

#[tokio::test]
async fn worker_loss_cannot_release_custody_around_a_non_holder_descendant() {
    let mut tree = tree_helper::EscapedNonHolderTree::spawn();
    let root = tree.root_identity();
    let leaf = tree.wait_for_leaf();
    let mut fixture = support::MacosHandshakeFixture::with_exact_child(None, root);
    let writer_handle = fixture.capability_writer_handle();
    assert_non_holder(root, writer_handle);
    assert_non_holder(leaf, writer_handle);
    assert_escaped(root, leaf);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = super::current(&fixture.host, &record);

    fixture.close_worker_aliases();
    assert_poisoned(&fixture.host, &lease).await;
    let gone_worker = super::reaped_process_identity();
    lease.probe_worker_identity(gone_worker);
    disconnect.send(()).unwrap();
    server.await.unwrap();
    super::wait_until(&fixture.host, || {
        fixture.host.request_drain.fail_stop_detail().is_some() && !lease.settlement_is_active()
    })
    .await;

    assert_eq!(lease.state(), SecondaryCustodyState::Disconnected);
    assert!(lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(PREMATURE_RELEASE_DETAIL)
    );
    assert!(Arc::ptr_eq(&lease, &super::current(&fixture.host, &record)));
    assert_eq!(root.observe(), ProcessIdentityObservation::Matching);
    assert_eq!(leaf.observe(), ProcessIdentityObservation::Matching);
    assert_poisoned(&fixture.host, &lease).await;
}

#[test]
fn escaped_non_holder_tree_helper() {
    tree_helper::run_role();
}

fn assert_non_holder(identity: ProcessIdentity, writer_handle: u64) {
    assert!(identity.matches());
    assert!(
        !crate::process::process_holds_capability_for_test(identity.pid, writer_handle).unwrap()
    );
}

fn assert_escaped(root: ProcessIdentity, leaf: ProcessIdentity) {
    let root_session = process_session(root);
    let leaf_session = process_session(leaf);
    assert_ne!(root_session, leaf_session);
    assert_eq!(leaf_session, libc::pid_t::try_from(leaf.pid).unwrap());
}

fn process_session(identity: ProcessIdentity) -> libc::pid_t {
    let pid = libc::pid_t::try_from(identity.pid).unwrap();
    let session = unsafe { libc::getsid(pid) };
    assert_ne!(session, PROCESS_QUERY_FAILURE);
    session
}

async fn assert_poisoned(host: &Host, lease: &SecondaryCustodyLease) {
    let detail = tokio::time::timeout(host.worker_ready_timeout, async {
        loop {
            match lease.platform_closure_settled() {
                Err(error) => break error.to_string(),
                Ok(false) => tokio::time::sleep(host.worker_ready_poll_interval).await,
                Ok(true) => panic!("{PREMATURE_RELEASE_MISCLASSIFIED_DETAIL}"),
            }
        }
    })
    .await
    .unwrap();
    assert!(detail.contains(PREMATURE_RELEASE_DETAIL));
}
