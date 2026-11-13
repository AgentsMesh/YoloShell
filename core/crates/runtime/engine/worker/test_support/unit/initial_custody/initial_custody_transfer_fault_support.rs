fn active_checkpoint(
    fixture: &Fixture,
    record: &RuntimeRecord,
) -> startup_checkpoint::ActiveStartupCheckpoint {
    const PRE_CUSTODY_STAGES: [startup_checkpoint::StartupStage; 8] = [
        startup_checkpoint::StartupStage::StartupContainment,
        startup_checkpoint::StartupStage::WorkerSocket,
        startup_checkpoint::StartupStage::AgentRuntime,
        startup_checkpoint::StartupStage::TerminalJournal,
        startup_checkpoint::StartupStage::Pty,
        startup_checkpoint::StartupStage::ChildProcess,
        startup_checkpoint::StartupStage::BootstrapGate,
        startup_checkpoint::StartupStage::RuntimeRecord,
    ];
    let launch = WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: record.terminal_id,
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        socket_path: record.socket_path.clone(),
        auth_token: record.auth_token.clone(),
        controller_generation: record.controller_generation,
        command: record.command.clone(),
        args: record.args.clone(),
        cwd: record.cwd.clone(),
        environment: Default::default(),
        initial_input: None,
        cols: record.cols,
        rows: record.rows,
        context: None,
    };
    crate::paths::atomic_write_json(
        &fixture.state.paths.worker_launch(record.terminal_id),
        &launch,
    )
    .unwrap();
    let mut checkpoint = startup_checkpoint::ActiveStartupCheckpoint::begin(
        &fixture.state.namespace,
        &fixture.state.paths,
        &launch,
        crate::process::ProcessIdentity {
            pid: record.worker_pid,
            start_time: record.worker_start_time,
        },
    )
    .unwrap();
    for stage in PRE_CUSTODY_STAGES {
        checkpoint
            .enter(&fixture.state.namespace, &fixture.state.paths, stage)
            .unwrap();
        if stage == startup_checkpoint::StartupStage::ChildProcess {
            checkpoint
                .complete_child(
                    &fixture.state.namespace,
                    &fixture.state.paths,
                    crate::process::ProcessIdentity {
                        pid: record.child_pid,
                        start_time: record.child_start_time,
                    },
                )
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.state.namespace, &fixture.state.paths, stage)
                .unwrap();
        }
    }
    checkpoint
        .enter(
            &fixture.state.namespace,
            &fixture.state.paths,
            startup_checkpoint::StartupStage::SecondaryCustody,
        )
        .unwrap();
    checkpoint
}

fn initial_fault_request(record: &RuntimeRecord) -> crate::pty_custody::CustodyRequest {
    crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        crate::pty_custody::CustodyRuntimeIdentity::from_record(record),
        crate::pty_custody::CustodyHolderIdentity::capture().unwrap(),
        record.auth_token.clone(),
        record.controller_generation,
    )
}

async fn drive_faulting_host(
    paths: Paths,
    request: crate::pty_custody::CustodyRequest,
    failure: InitialHandshakeFailure,
) -> Result<crate::pty_custody::CustodyOffer> {
    let path = paths.worker_socket(request.runtime.terminal_id)?;
    let mut stream = crate::pty_custody::CustodyUnixStream::connect(&path).await?;
    stream.write_opening(&request).await?;
    let offer: crate::pty_custody::CustodyOffer = stream.read().await?;
    if matches!(
        failure,
        InitialHandshakeFailure::Injected(InitialCustodyHandshakeBoundary::DescriptorSend)
    ) {
        return Ok(offer);
    }
    let descriptors = stream
        .receive_rights(offer.descriptor_set.descriptor_count())
        .await?;
    if matches!(
        failure,
        InitialHandshakeFailure::Injected(InitialCustodyHandshakeBoundary::PreparedRead)
    ) {
        return Ok(offer);
    }
    #[cfg(target_os = "macos")]
    let borrowed: Vec<_> = descriptors
        .iter()
        .map(|descriptor| std::os::fd::AsFd::as_fd(descriptor))
        .collect();
    #[cfg(target_os = "macos")]
    let prepared =
        initial_test_prepared_ack_with_descriptors(&paths, &request, &offer, &borrowed).await?;
    #[cfg(not(target_os = "macos"))]
    let prepared = crate::pty_custody::CustodyHostAck::prepared(&offer);
    stream.write(&prepared).await?;
    if !matches!(failure, InitialHandshakeFailure::ReadyValidation) {
        return Ok(offer);
    }
    let commit: crate::pty_custody::CustodyWorkerCommit = stream.read().await?;
    assert!(commit.matches_offer(&offer));
    stream
        .write(&crate::pty_custody::CustodyHostAck::ready(
            &offer,
            request
                .controller_generation
                .saturating_add(FORGED_READY_GENERATION_INCREMENT),
        ))
        .await?;
    Ok(offer)
}

fn assert_pending_host_release(
    transfer: &Arc<startup_custody_transfer::StartupCustodyTransferFence>,
) {
    let mut output = ExclusivePtyOutput::startup_custody_for_test();
    output
        .bind_custody_transfer_fence(Arc::clone(transfer))
        .unwrap();
    output.mark_owner_master_released();
    assert!(matches!(
        output.release_master_aliases().unwrap(),
        pty_output_progress::MasterAliasReleasePoll::Pending
    ));
}

fn assert_durable_boundary(
    fixture: &Fixture,
    request: &crate::pty_custody::CustodyRequest,
    offer: &crate::pty_custody::CustodyOffer,
    failure: InitialHandshakeFailure,
) {
    let durable: crate::pty_custody::CustodyDurableRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.pty_custody(fixture.terminal_id()))
        .unwrap();
    let matches = durable.armed_lease().is_some_and(|candidate| {
        candidate.holder == request.holder()
            && candidate.controller_generation == request.controller_generation
            && durable.matches_candidate(offer, candidate)
    });
    assert_eq!(matches, failure.durable_is_armed());
}
