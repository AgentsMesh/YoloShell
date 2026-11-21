use super::constants::*;
use super::fixture::connect_memory_with_timeout;
use super::memory_fs::MemoryFilesystem;
use crate::remote::cleanup_settlement_timeout;
use std::sync::Arc;

#[tokio::test]
async fn transferred_close_timeout_aborts_driver_only_after_transport_death() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    let (custody, aborted) = pending_custody(&fixture.connection);

    assert!(custody.close().await.is_err());
    assert!(aborted.await.unwrap());
    assert!(fixture.connection.session.is_closed());
}

#[tokio::test]
async fn registry_settlement_retains_transferred_driver_until_fail_stop() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    let (custody, aborted) = pending_custody(&fixture.connection);
    drop(custody);

    fixture
        .connection
        .cleanup
        .settle(cleanup_settlement_timeout(SHORT_OPERATION_TIMEOUT))
        .await
        .unwrap();
    assert!(aborted.await.unwrap());
    assert!(fixture.connection.session.is_closed());
}

#[tokio::test]
async fn registry_drop_detaches_transferred_fail_stop_without_aborting_it() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem.clone(), SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    let (custody, aborted) = pending_custody(&fixture.connection);
    drop(custody);
    drop(fixture);

    assert!(
        tokio::time::timeout(TEST_INACTIVITY_TIMEOUT, aborted)
            .await
            .unwrap()
            .unwrap()
    );
    tokio::time::timeout(TEST_INACTIVITY_TIMEOUT, filesystem.wait_connection_closed())
        .await
        .unwrap();
}

fn pending_custody(
    connection: &Arc<super::super::RusshConnection>,
) -> (
    super::super::channel_transfer_custody::TransferredChannelCustody,
    tokio::sync::oneshot::Receiver<bool>,
) {
    let (observed, aborted) = tokio::sync::oneshot::channel();
    let marker = AbortMarker {
        session: Arc::clone(&connection.session),
        observed: Some(observed),
    };
    let task = tokio::spawn(async move {
        let _marker = marker;
        std::future::pending::<()>().await;
        Ok(())
    });
    let custody = super::super::channel_transfer_custody::TransferredChannelCustody::from_driver_task_for_test(
        Arc::clone(&connection.session),
        connection.cleanup.reserve().unwrap(),
        SHORT_OPERATION_TIMEOUT,
        task,
    );
    (custody, aborted)
}

struct AbortMarker {
    session: Arc<russh::client::Handle<super::super::HostKeyHandler>>,
    observed: Option<tokio::sync::oneshot::Sender<bool>>,
}

impl Drop for AbortMarker {
    fn drop(&mut self) {
        if let Some(observed) = self.observed.take() {
            let _ = observed.send(self.session.is_closed());
        }
    }
}
