const UNCERTAIN_EXEC_HOST_ADMISSION_DETAIL: &str =
    "uncertain-exec Host could not complete pre-transfer admission";
const UNCERTAIN_EXEC_HOST_OFFER_DETAIL: &str =
    "uncertain-exec Host received an invalid custody offer";

async fn connect_uncertain_exec_host(
    record: &RuntimeRecord,
    request: &crate::pty_custody::CustodyRequest,
) -> (
    crate::pty_custody::CustodyUnixStream,
    crate::pty_custody::CustodyOffer,
) {
    let deadline = tokio::time::Instant::now() + UNCERTAIN_EXEC_HOST_WAIT_TIMEOUT;
    loop {
        let error = match attempt_uncertain_exec_host_admission(record, request).await {
            Ok(admitted) => return admitted,
            Err(error) => error,
        };
        assert!(
            tokio::time::Instant::now() < deadline,
            "{UNCERTAIN_EXEC_HOST_ADMISSION_DETAIL}: {error:#}"
        );
        tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
    }
}

async fn attempt_uncertain_exec_host_admission(
    record: &RuntimeRecord,
    request: &crate::pty_custody::CustodyRequest,
) -> Result<(
    crate::pty_custody::CustodyUnixStream,
    crate::pty_custody::CustodyOffer,
)> {
    let mut stream = crate::pty_custody::CustodyUnixStream::connect(&record.socket_path).await?;
    stream.write_opening(request).await?;
    let offer: crate::pty_custody::CustodyOffer = stream.read().await?;
    anyhow::ensure!(offer.matches_request(request), UNCERTAIN_EXEC_HOST_OFFER_DETAIL);
    Ok((stream, offer))
}
