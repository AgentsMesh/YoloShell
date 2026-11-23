const RUNTIME_ERROR_CUSTODY_HOLDER_DETAIL: &str = "capture runtime-error Host custody identity";
const RUNTIME_ERROR_CUSTODY_CONNECT_DETAIL: &str = "connect runtime-error custody transport";
const RUNTIME_ERROR_CUSTODY_OPENING_DETAIL: &str = "write runtime-error custody opening";
const RUNTIME_ERROR_CUSTODY_OFFER_DETAIL: &str = "read runtime-error custody offer";
const RUNTIME_ERROR_CUSTODY_OFFER_MISMATCH_DETAIL: &str =
    "runtime-error custody offer did not match the request";
const RUNTIME_ERROR_CUSTODY_RIGHTS_DETAIL: &str = "receive runtime-error custody descriptors";
const RUNTIME_ERROR_CUSTODY_PREPARED_DETAIL: &str =
    "write runtime-error custody prepared acknowledgement";
const RUNTIME_ERROR_CUSTODY_COMMIT_DETAIL: &str = "read runtime-error custody commit";
const RUNTIME_ERROR_CUSTODY_COMMIT_MISMATCH_DETAIL: &str =
    "runtime-error custody commit did not match the offer";
const RUNTIME_ERROR_CUSTODY_READY_DETAIL: &str =
    "write runtime-error custody ready acknowledgement";
const RUNTIME_ERROR_CUSTODY_DISPOSITION_DETAIL: &str =
    "read runtime-error custody ready disposition";
const RUNTIME_ERROR_CUSTODY_RELEASE_DETAIL: &str =
    "runtime-error Worker activated custody instead of rolling it back";
const RUNTIME_ERROR_CUSTODY_RELEASE_MISMATCH_DETAIL: &str =
    "runtime-error custody release did not match the offer";
const RUNTIME_ERROR_CUSTODY_RELEASE_ACK_DETAIL: &str =
    "construct runtime-error custody release acknowledgement";
const RUNTIME_ERROR_CUSTODY_RELEASE_WRITE_DETAIL: &str =
    "write runtime-error custody release acknowledgement";
const RUNTIME_ERROR_CUSTODY_PROTOCOL_TIMEOUT_DETAIL: &str =
    "runtime-error custody protocol did not finish before the test deadline";

async fn complete_runtime_error_custody(
    paths: &Paths,
    terminal_id: Uuid,
    worker: &mut RuntimeErrorWorker,
) {
    super::runtime_result_test_hook::watch(terminal_id);
    let record = match wait_for_runtime_error_record(paths, terminal_id).await {
        Ok(record) => record,
        Err(error) => worker.protocol_failure(terminal_id, error).await,
    };
    let protocol = tokio::time::timeout(
        RUNTIME_ERROR_CUSTODY_TIMEOUT,
        complete_runtime_error_custody_protocol(paths, &record),
    )
    .await
    .context(RUNTIME_ERROR_CUSTODY_PROTOCOL_TIMEOUT_DETAIL)
    .and_then(|protocol| protocol);
    if let Err(error) = protocol {
        worker.protocol_failure(terminal_id, error).await;
    }
    super::runtime_result_test_hook::take(terminal_id);
}

async fn complete_runtime_error_custody_protocol(
    paths: &Paths,
    record: &RuntimeRecord,
) -> Result<()> {
    let holder = crate::pty_custody::CustodyHolderIdentity::capture()
        .context(RUNTIME_ERROR_CUSTODY_HOLDER_DETAIL)?;
    let request = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        crate::pty_custody::CustodyRuntimeIdentity::from_record(record),
        holder,
        record.auth_token.clone(),
        record.controller_generation,
    );
    let mut stream = crate::pty_custody::CustodyUnixStream::connect(&record.socket_path)
        .await
        .context(RUNTIME_ERROR_CUSTODY_CONNECT_DETAIL)?;
    stream
        .write_opening(&request)
        .await
        .context(RUNTIME_ERROR_CUSTODY_OPENING_DETAIL)?;
    let offer: crate::pty_custody::CustodyOffer = stream
        .read()
        .await
        .context(RUNTIME_ERROR_CUSTODY_OFFER_DETAIL)?;
    anyhow::ensure!(
        offer.matches_request(&request),
        RUNTIME_ERROR_CUSTODY_OFFER_MISMATCH_DETAIL
    );
    let descriptors = stream
        .receive_rights(offer.descriptor_set.descriptor_count())
        .await
        .context(RUNTIME_ERROR_CUSTODY_RIGHTS_DETAIL)?;
    let custody = import_runtime_error_custody(record, &offer, descriptors)?;
    let prepared = initial_test_prepared_ack(paths, &request, &offer, &custody).await?;
    stream
        .write(&prepared)
        .await
        .context(RUNTIME_ERROR_CUSTODY_PREPARED_DETAIL)?;
    let commit: crate::pty_custody::CustodyWorkerCommit = stream
        .read()
        .await
        .context(RUNTIME_ERROR_CUSTODY_COMMIT_DETAIL)?;
    anyhow::ensure!(
        commit.matches_offer(&offer),
        RUNTIME_ERROR_CUSTODY_COMMIT_MISMATCH_DETAIL
    );
    stream
        .write(&crate::pty_custody::CustodyHostAck::ready(
            &offer,
            request.controller_generation,
        ))
        .await
        .context(RUNTIME_ERROR_CUSTODY_READY_DETAIL)?;
    let disposition: crate::pty_custody::CustodyWorkerReadyDisposition = stream
        .read()
        .await
        .context(RUNTIME_ERROR_CUSTODY_DISPOSITION_DETAIL)?;
    let crate::pty_custody::CustodyWorkerReadyDisposition::Release { control: release } =
        disposition
    else {
        anyhow::bail!(RUNTIME_ERROR_CUSTODY_RELEASE_DETAIL);
    };
    anyhow::ensure!(
        release.matches_release(&offer),
        RUNTIME_ERROR_CUSTODY_RELEASE_MISMATCH_DETAIL
    );
    await_runtime_error_custody_settlement(record, &custody).await?;
    drop(custody);
    let acknowledgement = crate::pty_custody::CustodyControlAck::released(&release)
        .context(RUNTIME_ERROR_CUSTODY_RELEASE_ACK_DETAIL)?;
    stream
        .write(&acknowledgement)
        .await
        .context(RUNTIME_ERROR_CUSTODY_RELEASE_WRITE_DETAIL)
}
