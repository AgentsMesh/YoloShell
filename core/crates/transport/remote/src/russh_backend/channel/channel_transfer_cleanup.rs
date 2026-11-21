async fn close_driver(
    session: Arc<Handle<HostKeyHandler>>,
    close: Option<oneshot::Sender<()>>,
    mut task: DriverTask,
    timeout: Duration,
) -> Result<(), RemoteError> {
    if let Some(close) = close {
        let _ = close.send(());
    }
    match tokio::time::timeout(timeout, &mut task).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(primary))) => {
            #[cfg(all(test, feature = "yolo-integration-tests"))]
            super::connection_diagnostics::driver_error(&primary);
            settle_escalation(primary, disconnect_retained(&session, timeout).await)
        }
        Ok(Err(error)) => {
            #[cfg(all(test, feature = "yolo-integration-tests"))]
            super::connection_diagnostics::join_error(&error);
            let primary = ssh_error(format!(
                "{SFTP_CHANNEL_DRIVER_TASK_FAILURE_CONTEXT}: {error}"
            ));
            settle_escalation(primary, disconnect_retained(&session, timeout).await)
        }
        Err(_) => {
            #[cfg(all(test, feature = "yolo-integration-tests"))]
            super::connection_diagnostics::cleanup_timeout(timeout);
            disconnect_and_abort(session, Some(task), timeout).await
        }
    }
}

async fn disconnect_and_abort_pair<Output>(
    session: Arc<Handle<HostKeyHandler>>,
    driver: DriverTask,
    task: JoinHandle<Output>,
    timeout: Duration,
) -> Result<(), RemoteError>
where
    Output: Send + 'static,
{
    let disconnected = disconnect_retained(&session, timeout).await;
    let (driver, task) = tokio::join!(abort_task(driver), abort_task(task));
    combine(disconnected, combine(driver, task))
}

async fn abort_task<Output>(task: JoinHandle<Output>) -> Result<(), RemoteError> {
    task.abort();
    tokio::time::timeout(SSH_CLEANUP_ABORT_SETTLEMENT_TIMEOUT, task)
        .await
        .map(|_| ())
        .map_err(|_| Error::transport(SFTP_CHANNEL_ABORT_SETTLEMENT_MESSAGE))
}

fn combine(
    primary: Result<(), RemoteError>,
    settlement: Result<(), RemoteError>,
) -> Result<(), RemoteError> {
    match (primary, settlement) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(primary), Err(settlement)) => Err(RemoteError::LifecycleSettlement {
            primary: Box::new(primary),
            settlement: Box::new(settlement),
        }),
    }
}
