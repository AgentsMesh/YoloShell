use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn indeterminate_prior_holder_cannot_receive_offer_or_rearm_custody() {
    let fixture = Fixture::new();
    let record = lock_recover(&fixture.state.record).clone();
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let durable = armed_custody_for_test(&record, holder);
    let custody_path = fixture.state.paths.pty_custody(record.terminal_id);
    fixture
        .state
        .namespace
        .write_json(&custody_path, &durable)
        .unwrap();
    let (_, durable_identity): (crate::pty_custody::CustodyDurableRecord, _) = fixture
        .state
        .namespace
        .read_json_with_identity(&custody_path)
        .unwrap();
    let controller_before = fixture.state.controller_generation.load(Ordering::Acquire);
    let authority_before = fixture.state.secondary_custody_authority.active_for_test();
    let (source, _pair, _writer) = custody_source_for_test(&record);
    let (worker, peer) = UnixStream::pair().unwrap();
    let worker = crate::pty_custody::CustodyUnixStream::from_local(worker).unwrap();
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    let successor = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
        holder,
        record.auth_token.clone(),
        controller_before + CUSTODY_SUCCESSOR_INCREMENT,
    );
    let _unknown = crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        crate::process::ProcessIdentityRelation::Unknown,
    );

    let mut adoption_fence =
        secondary_custody_adoption_fence::CustodyAdoptionFence::unarmed(Arc::clone(&fixture.state));
    let publication = fixture.state.reserve_response_publication().unwrap();
    let result = secondary_custody_handshake::exchange_successor(
        worker,
        successor,
        &fixture.state.namespace,
        &fixture.state.paths,
        &record,
        &durable,
        durable_identity,
        secondary_custody_handshake_source::WorkerCustodyHandshakeSource::Initial {
            source: &source,
            profile: crate::pty_custody::CustodyProfile::ActiveRunning,
        },
        &mut adoption_fence,
        publication,
    )
    .await;

    assert!(result.is_err());
    let wire_error = peer
        .read::<crate::pty_custody::CustodyOffer>()
        .await
        .unwrap_err();
    assert!(matches!(
        wire_error.downcast_ref::<ProtocolError>(),
        Some(ProtocolError::EndOfStream)
    ));
    let persisted: crate::pty_custody::CustodyDurableRecord =
        fixture.state.namespace.read_json(&custody_path).unwrap();
    assert_eq!(persisted, durable);
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        controller_before
    );
    assert_eq!(
        fixture.state.secondary_custody_authority.active_for_test(),
        authority_before
    );
}
