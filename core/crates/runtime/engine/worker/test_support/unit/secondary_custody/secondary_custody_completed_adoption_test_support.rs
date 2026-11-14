#[cfg(target_os = "macos")]
mod completed_adoption_test_support {
    use super::*;

    const GENERATION_OVERFLOW_DETAIL: &str = "completed adoption generation overflow";
    const PREDECESSOR_IDENTITY_OVERFLOW_DETAIL: &str =
        "completed adoption predecessor identity overflow";
    const RELEASE_CLOSE_REENTRY_DETAIL: &str =
        "finalized predecessor release retried local PTY closure";
    const RELEASE_FINALIZATION_TIMEOUT_DETAIL: &str =
        "predecessor PTY release did not reach Finalized";
    const RELEASE_HOLD_TIMEOUT_DETAIL: &str =
        "predecessor PTY custody did not settle after final release";

    pub(super) struct CompletedAdoption {
        pub(super) generation: u64,
        pub(super) peer: crate::pty_custody::CustodyUnixStream,
        pub(super) task: tokio::task::JoinHandle<Result<()>>,
    }

    impl SuccessorAdoptionWire {
        fn install_after_finalized_release(fixture: &Fixture) -> Self {
            let record = lock_recover(&fixture.state.record).clone();
            let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
            let predecessor = crate::pty_custody::CustodyHolderIdentity {
                pid: holder.pid,
                start_time: holder
                    .start_time
                    .checked_add(CUSTODY_SUCCESSOR_INCREMENT)
                    .expect(PREDECESSOR_IDENTITY_OVERFLOW_DETAIL),
            };
            let gone = crate::process::install_test_process_identity_relation(
                predecessor.pid,
                predecessor.start_time,
                crate::process::ProcessIdentityRelation::Gone,
            );
            let durable = armed_custody_for_active_predecessor(fixture, &record, predecessor);
            fixture
                .state
                .namespace
                .write_json(
                    &fixture.state.paths.pty_custody(record.terminal_id),
                    &durable,
                )
                .unwrap();
            assert!(lock_recover(&fixture.state.secondary_custody).is_none());
            let (worker, peer) = UnixStream::pair().unwrap();
            let generation = record
                .controller_generation
                .checked_add(CUSTODY_SUCCESSOR_INCREMENT)
                .expect(GENERATION_OVERFLOW_DETAIL);
            let request = crate::pty_custody::CustodyRequest::new(
                crate::pty_custody::CustodyLeaseId::new(),
                crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
                holder,
                record.auth_token,
                generation,
            );
            Self {
                state: Arc::clone(&fixture.state),
                worker: Some(worker),
                peer: Some(crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap()),
                request,
                received_descriptors: Vec::new(),
                pair: None,
                writer: None,
                _gone: Some(gone),
            }
        }
    }

    pub(super) async fn finalize_predecessor_release(fixture: &Fixture) {
        let identity = fixture
            .state
            .secondary_custody_authority
            .active_for_test()
            .unwrap();
        let (lease, peer) = test_custody_lease(fixture, identity);
        let expected_offer = lease.offer_for_test();
        fixture
            .state
            .pty_alias_custody
            .install_release_handle(lease.release_handle());
        let hold = tokio::spawn(lease.hold(Arc::clone(&fixture.state)));
        let state = Arc::clone(&fixture.state);
        let permit = state.admit_system_mutation().await;
        let closure = state.prepare_system_alias_release(&permit);
        state.commit_system_alias_release(&permit, closure);
        drop(permit);
        assert_eq!(
            state.pty_alias_custody.close_local_aliases().unwrap(),
            pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Closed
        );
        let authority = runtime_release_authority(&lock_recover(&state.record));
        assert!(matches!(
            state
                .pty_alias_custody
                .release(|| {
                    authority
                        .take_local_aliases_closed()
                        .map(pty_runtime_alias_custody::PtyRuntimeLocalAliasRelease::closed)
                })
                .unwrap(),
            pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending
        ));
        let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
        let control: crate::pty_custody::CustodyControl = peer.read().await.unwrap();
        assert!(control.matches_release(&expected_offer));
        peer.write(&crate::pty_custody::CustodyControlAck::released(&control).unwrap())
            .await
            .unwrap();

        let deadline = Instant::now() + CUSTODY_RELEASE_TIMEOUT;
        loop {
            match state
                .pty_alias_custody
                .release(|| -> anyhow::Result<_> { panic!("{RELEASE_CLOSE_REENTRY_DETAIL}") })
            {
                Ok(pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Closed(_)) => break,
                Ok(pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending)
                    if Instant::now() < deadline =>
                {
                    tokio::task::yield_now().await;
                }
                _ => panic!("{RELEASE_FINALIZATION_TIMEOUT_DETAIL}"),
            }
        }
        assert!(matches!(
            state
                .pty_alias_custody
                .release(|| -> anyhow::Result<_> { panic!("{RELEASE_CLOSE_REENTRY_DETAIL}") }),
            Ok(pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Delivered)
        ));
        drop(peer);
        tokio::time::timeout(CUSTODY_RELEASE_TIMEOUT, hold)
            .await
            .expect(RELEASE_HOLD_TIMEOUT_DETAIL)
            .unwrap()
            .unwrap();
    }

    pub(super) async fn adopt_completed_runtime(fixture: &Fixture) -> CompletedAdoption {
        let mut wire = SuccessorAdoptionWire::install_after_finalized_release(fixture);
        let generation = wire.request.controller_generation;
        let task = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
        let offer = wire.receive_offer().await;
        assert_eq!(
            offer.descriptor_set,
            crate::pty_custody::CustodyDescriptorSet::ReleasedSuccessor
        );
        assert!(wire.received_descriptors.is_empty());
        wire.receive_commit(&offer).await;
        wire.send_ready(&offer).await;
        let mut peer = wire.take_peer();
        receive_exact_worker_activation(&mut peer, &offer, generation).await;
        CompletedAdoption {
            generation,
            peer,
            task,
        }
    }
}

#[cfg(target_os = "macos")]
use completed_adoption_test_support::*;
