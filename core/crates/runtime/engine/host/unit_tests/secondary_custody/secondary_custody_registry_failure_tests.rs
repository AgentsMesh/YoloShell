use super::*;
use crate::pty_custody::{CustodyRequest, CustodyRuntimeIdentity};

#[tokio::test]
async fn supersede_rejects_a_missing_registry_without_dropping_custody() {
    let (fixture, lease, server, finish) = ready_fixture().await;
    *crate::host::lock_recover(&lease.registry) = None;

    assert_protocol_failure(lease.commit_superseded());
    assert_active_custody(&fixture, &lease);

    finish_server(server, finish).await;
}

#[tokio::test]
async fn supersede_rejects_a_missing_exact_active_entry() {
    let (fixture, lease, server, finish) = ready_fixture().await;
    let key = SecondaryCustodyKey::from_runtime(&lease.runtime);
    let removed = crate::host::lock_recover(&fixture.host.secondary_custody.entries.values)
        .active
        .remove(&key)
        .unwrap();

    assert_protocol_failure(lease.commit_superseded());
    assert!(Arc::ptr_eq(&removed, &lease));
    assert_retained_custody(&lease);

    finish_server(server, finish).await;
}

#[tokio::test]
async fn supersede_rejects_an_exact_entry_pointer_mismatch() {
    let (fixture, lease, server, finish) = ready_fixture().await;
    let (replacement, _peer) = replacement_lease(&fixture.host, &lease);
    let key = SecondaryCustodyKey::from_runtime(&lease.runtime);
    let original = crate::host::lock_recover(&fixture.host.secondary_custody.entries.values)
        .active
        .insert(key, Arc::clone(&replacement))
        .unwrap();

    assert_protocol_failure(lease.commit_superseded());
    assert!(Arc::ptr_eq(&original, &lease));
    assert_retained_custody(&lease);
    assert!(Arc::ptr_eq(
        &fixture
            .host
            .secondary_custody
            .current(&lease.runtime)
            .unwrap(),
        &replacement
    ));

    finish_server(server, finish).await;
}

#[tokio::test]
async fn install_rejects_occupied_active_and_retired_exact_entries() {
    let (fixture, lease, server, finish) = ready_fixture().await;
    let (active_replacement, _active_peer) = replacement_lease(&fixture.host, &lease);

    assert_protocol_failure(fixture.host.secondary_custody.install(active_replacement));
    lease.commit_superseded().unwrap();
    let (retired_replacement, _retired_peer) = replacement_lease(&fixture.host, &lease);
    let runtime = lease.runtime.clone();
    let retired = Arc::downgrade(&lease);
    drop(lease);

    assert!(retired.upgrade().is_none());
    assert_protocol_failure(fixture.host.secondary_custody.install(retired_replacement));
    assert!(matches!(
        fixture.host.secondary_custody.admission(&runtime),
        SecondaryCustodyAdmission::Retired {
            state: SecondaryCustodyState::Superseded,
            ..
        }
    ));
    assert!(
        fixture
            .host
            .secondary_custody
            .retired_for_settlement(&runtime)
    );

    finish_server(server, finish).await;
}

async fn ready_fixture() -> (
    support::MacosHandshakeFixture,
    Arc<SecondaryCustodyLease>,
    tokio::task::JoinHandle<()>,
    tokio::sync::oneshot::Sender<()>,
) {
    super::release_commit_tests::ready_fixture().await
}

fn replacement_lease(
    host: &Host,
    lease: &SecondaryCustodyLease,
) -> (
    Arc<SecondaryCustodyLease>,
    crate::local_unix::LocalUnixStream,
) {
    let request = CustodyRequest::new(
        lease.offer.lease_id,
        lease.runtime.clone(),
        lease.holder,
        TEST_TOKEN.to_owned(),
        lease.controller_generation,
    );
    let (stream, peer) = crate::local_unix::LocalUnixStream::pair().unwrap();
    let stream = CustodyUnixStream::from_local(stream).unwrap();
    let replacement = SecondaryCustodyLease::new(
        host,
        &request,
        lease.offer.clone(),
        None,
        PlatformSecondaryCustody::ReleasedSuccessor,
        stream,
    )
    .unwrap();
    (Arc::new(replacement), peer)
}

fn assert_active_custody(
    fixture: &support::MacosHandshakeFixture,
    lease: &Arc<SecondaryCustodyLease>,
) {
    assert_retained_custody(lease);
    let current = fixture
        .host
        .secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(&fixture.record))
        .unwrap();
    assert!(Arc::ptr_eq(&current, lease));
}

fn assert_retained_custody(lease: &SecondaryCustodyLease) {
    assert_eq!(lease.state(), SecondaryCustodyState::Ready);
    assert!(lease.is_ready());
    assert!(lease.retains_platform());
}

fn assert_protocol_failure<T>(result: Result<T, WorkerRpcError>) {
    assert!(matches!(result, Err(WorkerRpcError::Protocol(_))));
}

async fn finish_server(
    server: tokio::task::JoinHandle<()>,
    finish: tokio::sync::oneshot::Sender<()>,
) {
    finish.send(()).unwrap();
    server.await.unwrap();
}
