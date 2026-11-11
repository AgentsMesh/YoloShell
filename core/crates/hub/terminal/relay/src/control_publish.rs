pub(crate) async fn publish(
    status: &mpsc::Sender<Status>,
    event: Status,
) -> Result<(), RelayError> {
    tokio::time::timeout(OUTPUT_TIMEOUT, status.send(event))
        .await
        .map_err(|_| RelayError::StatusClosed)?
        .map_err(|_| RelayError::StatusClosed)
}
