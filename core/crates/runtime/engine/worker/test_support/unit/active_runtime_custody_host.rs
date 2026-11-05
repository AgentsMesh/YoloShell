const ACTIVE_CUSTODY_ACTIVATION_DETAIL: &str =
    "active test Host received a pre-activation custody release";
const ACTIVE_CUSTODY_RELEASE_DETAIL: &str = "active test Host received an invalid custody release";
const ACTIVE_CUSTODY_ACK_DETAIL: &str =
    "construct active test Host custody release acknowledgement";

async fn hold_active_runtime_custody(paths: &Paths, terminal_id: Uuid) -> Result<()> {
    let record = wait_for_runtime_error_record(paths, terminal_id).await?;
    let request = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
        crate::pty_custody::CustodyHolderIdentity::capture()?,
        record.auth_token.clone(),
        record.controller_generation,
    );
    let mut stream = crate::pty_custody::CustodyUnixStream::connect(&record.socket_path).await?;
    stream.write_opening(&request).await?;
    let offer: crate::pty_custody::CustodyOffer = stream.read().await?;
    anyhow::ensure!(
        offer.matches_request(&request),
        ACTIVE_CUSTODY_RELEASE_DETAIL
    );
    let descriptors = stream
        .receive_rights(offer.descriptor_set.descriptor_count())
        .await?;
    let custody = import_runtime_error_custody(&record, &offer, descriptors)?;
    let prepared = initial_test_prepared_ack(paths, &request, &offer, &custody).await?;
    stream.write(&prepared).await?;
    let commit: crate::pty_custody::CustodyWorkerCommit = stream.read().await?;
    anyhow::ensure!(commit.matches_offer(&offer), ACTIVE_CUSTODY_RELEASE_DETAIL);
    stream
        .write(&crate::pty_custody::CustodyHostAck::ready(
            &offer,
            request.controller_generation,
        ))
        .await?;
    let disposition: crate::pty_custody::CustodyWorkerReadyDisposition = stream.read().await?;
    let crate::pty_custody::CustodyWorkerReadyDisposition::Activated { activation } = disposition
    else {
        anyhow::bail!(ACTIVE_CUSTODY_ACTIVATION_DETAIL);
    };
    activation.into_exact(&offer, request.controller_generation)?;

    let release: crate::pty_custody::CustodyControl = stream.read().await?;
    anyhow::ensure!(
        release.matches_release(&offer),
        ACTIVE_CUSTODY_RELEASE_DETAIL
    );
    await_runtime_error_custody_settlement(&record, &custody).await?;
    drop(custody);
    let acknowledgement = crate::pty_custody::CustodyControlAck::released(&release)
        .context(ACTIVE_CUSTODY_ACK_DETAIL)?;
    stream.write(&acknowledgement).await
}

fn spawn_active_runtime_custody(
    paths: Paths,
    terminal_id: Uuid,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move { hold_active_runtime_custody(&paths, terminal_id).await })
}

fn spawn_active_runtime_custody_thread(
    paths: Paths,
    terminal_id: Uuid,
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(hold_active_runtime_custody(&paths, terminal_id))
    })
}
