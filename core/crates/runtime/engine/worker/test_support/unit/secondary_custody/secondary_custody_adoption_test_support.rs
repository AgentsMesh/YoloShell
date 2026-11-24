#[cfg(target_os = "macos")]
const ADOPTION_TEST_REQUEST_ID: &str = "successor-adoption-test";
#[cfg(target_os = "macos")]
const ADOPTION_TEST_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(target_os = "macos")]
const ADOPTION_TEST_SHORT_TIMEOUT: Duration = Duration::from_millis(500);
#[cfg(target_os = "macos")]
const ADOPTION_EXPECTED_ACTIVATION_DETAIL: &str =
    "successor adoption unexpectedly emitted a release decision";
#[cfg(target_os = "macos")]
const ADOPTION_EXPECTED_RELEASE_DETAIL: &str =
    "successor rollback unexpectedly emitted an activation decision";

#[cfg(target_os = "macos")]
struct SuccessorAdoptionWire {
    state: Arc<WorkerState>,
    worker: Option<UnixStream>,
    peer: Option<crate::pty_custody::CustodyUnixStream>,
    request: crate::pty_custody::CustodyRequest,
    received_descriptors: Vec<std::os::fd::OwnedFd>,
    pair: Option<portable_pty::PtyPair>,
    writer: Option<crate::process::MacosProcessCapabilityChildWriter>,
    _gone: Option<crate::process::ProcessIdentityRelationGuard>,
}

#[cfg(target_os = "macos")]
async fn receive_exact_worker_activation(
    peer: &mut crate::pty_custody::CustodyUnixStream,
    offer: &crate::pty_custody::CustodyOffer,
    generation: u64,
) {
    let disposition: crate::pty_custody::CustodyWorkerReadyDisposition = peer.read().await.unwrap();
    let crate::pty_custody::CustodyWorkerReadyDisposition::Activated { activation } = disposition
    else {
        panic!("{ADOPTION_EXPECTED_ACTIVATION_DETAIL}");
    };
    activation.into_exact(offer, generation).unwrap();
}

#[cfg(target_os = "macos")]
async fn acknowledge_exact_worker_release(
    peer: &mut crate::pty_custody::CustodyUnixStream,
    offer: &crate::pty_custody::CustodyOffer,
) {
    let disposition: crate::pty_custody::CustodyWorkerReadyDisposition = peer.read().await.unwrap();
    let crate::pty_custody::CustodyWorkerReadyDisposition::Release { control } = disposition else {
        panic!("{ADOPTION_EXPECTED_RELEASE_DETAIL}");
    };
    assert!(control.matches_release(offer));
    let acknowledgement = crate::pty_custody::CustodyControlAck::released(&control).unwrap();
    peer.write(&acknowledgement).await.unwrap();
}

#[cfg(target_os = "macos")]
impl SuccessorAdoptionWire {
    fn install(fixture: &Fixture) -> Self {
        let record = lock_recover(&fixture.state.record).clone();
        let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
        let predecessor = crate::pty_custody::CustodyHolderIdentity {
            pid: holder.pid,
            start_time: holder
                .start_time
                .saturating_add(CUSTODY_SUCCESSOR_INCREMENT),
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
        let (source, pair, writer) = custody_source_for_test(&record);
        let previous = lock_recover(&fixture.state.secondary_custody).replace(Arc::new(source));
        assert!(previous.is_none());
        let (worker, peer) = UnixStream::pair().unwrap();
        let request = crate::pty_custody::CustodyRequest::new(
            crate::pty_custody::CustodyLeaseId::new(),
            crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
            holder,
            record.auth_token,
            record
                .controller_generation
                .saturating_add(CUSTODY_SUCCESSOR_INCREMENT),
        );
        Self {
            state: Arc::clone(&fixture.state),
            worker: Some(worker),
            peer: Some(crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap()),
            request,
            received_descriptors: Vec::new(),
            pair: Some(pair),
            writer: Some(writer),
            _gone: Some(gone),
        }
    }

    async fn install_released(fixture: &Fixture) -> Self {
        let mut wire = Self::install(fixture);
        drop(lock_recover(&fixture.state.secondary_custody).take());
        drop(wire.pair.take());
        drop(wire.writer.take());
        let state = Arc::clone(&fixture.state);
        let custody = Arc::clone(&state.pty_alias_custody);
        let authority = runtime_release_authority(&lock_recover(&state.record));
        let permit = state.admit_system_mutation().await;
        let closure = state.prepare_system_alias_release(&permit);
        state.commit_system_alias_release(&permit, closure);
        drop(permit);
        custody.close_local_aliases().unwrap();
        let prepared = custody
            .release(|| {
                authority
                    .take_local_aliases_closed()
                    .map(pty_runtime_alias_custody::PtyRuntimeLocalAliasRelease::closed)
            })
            .unwrap();
        assert!(matches!(
            prepared,
            pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending
        ));
        wire
    }

    fn spawn(
        &mut self,
        state: Arc<WorkerState>,
        timeout: Duration,
    ) -> tokio::task::JoinHandle<Result<()>> {
        let opening = RequestEnvelope::new(
            ADOPTION_TEST_REQUEST_ID,
            crate::pty_custody::METHOD,
            to_value(&self.request).unwrap(),
        );
        let worker = self.worker.take().unwrap();
        let publication = state.reserve_response_publication().unwrap();
        tokio::spawn(secondary_custody_adoption::adopt_runtime_with_timeout(
            worker,
            opening,
            state,
            publication,
            timeout,
        ))
    }

    async fn receive_offer(&mut self) -> crate::pty_custody::CustodyOffer {
        let peer = self.peer.as_mut().unwrap();
        let offer: crate::pty_custody::CustodyOffer = peer.read().await.unwrap();
        assert!(offer.matches_request(&self.request));
        self.received_descriptors = peer
            .receive_rights(offer.descriptor_set.descriptor_count())
            .await
            .unwrap();
        assert_eq!(
            self.received_descriptors.len(),
            offer.descriptor_set.descriptor_count()
        );
        offer
    }

    async fn send_ready(&mut self, offer: &crate::pty_custody::CustodyOffer) {
        self.peer
            .as_mut()
            .unwrap()
            .write(&crate::pty_custody::CustodyHostAck::ready(
                offer,
                self.request.controller_generation,
            ))
            .await
            .unwrap();
    }

    fn take_peer(&mut self) -> crate::pty_custody::CustodyUnixStream {
        self.peer.take().unwrap()
    }
}

#[cfg(target_os = "macos")]
async fn await_adoption_failure(task: tokio::task::JoinHandle<Result<()>>) {
    let result = tokio::time::timeout(ADOPTION_TEST_TIMEOUT, task)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
}
