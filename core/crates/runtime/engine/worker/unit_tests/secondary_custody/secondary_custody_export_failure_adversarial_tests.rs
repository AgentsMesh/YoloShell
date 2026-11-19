use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn disconnected_successor_send_does_not_create_a_false_orphan() {
    let fixture = Fixture::new();
    let (source, _pair, _writer) = install_successor_export_source(&fixture);
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let mut export = source.lease_successor_export().unwrap();
    let (worker, peer) = crate::local_unix::LocalUnixStream::pair().unwrap();
    let worker = crate::pty_custody::CustodyUnixStream::from_local(worker).unwrap();
    drop(peer);

    assert_eq!(
        source.successor_export_counts_for_test(),
        SINGLE_ACTIVE_EXPORT
    );
    assert!(export.send_descriptors(&worker, holder).await.is_err());
    drop(export);

    assert_failed_export_settled(&fixture, &source, holder).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn failed_successor_send_with_connected_peer_does_not_create_an_orphan() {
    let fixture = Fixture::new();
    let (source, _pair, _writer) = install_successor_export_source(&fixture);
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let mut export = source.lease_successor_export().unwrap();
    let (worker, peer) = crate::local_unix::LocalUnixStream::pair().unwrap();
    shutdown_successor_export_writes(&worker);
    let worker = crate::pty_custody::CustodyUnixStream::from_local(worker).unwrap();

    assert!(export.send_descriptors(&worker, holder).await.is_err());
    drop(export);

    assert_failed_export_settled(&fixture, &source, holder).await;
    drop(peer);
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn cancelled_successor_send_before_writability_does_not_create_an_orphan() {
    let fixture = Fixture::new();
    let (source, _pair, _writer) = install_successor_export_source(&fixture);
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let mut export = source.lease_successor_export().unwrap();
    let (worker, peer) = crate::local_unix::LocalUnixStream::pair().unwrap();
    saturate_successor_export_socket(&worker);
    let worker = crate::pty_custody::CustodyUnixStream::from_local(worker).unwrap();
    let send = tokio::spawn(async move { export.send_descriptors(&worker, holder).await });

    assert_task_pending(&send).await;
    assert_eq!(
        source.successor_export_counts_for_test(),
        SINGLE_ACTIVE_EXPORT
    );
    send.abort();
    let cancellation = send.await.unwrap_err();
    assert!(cancellation.is_cancelled());

    assert_failed_export_settled(&fixture, &source, holder).await;
    drop(peer);
}
