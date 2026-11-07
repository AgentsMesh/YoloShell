#[cfg(target_os = "macos")]
impl SuccessorAdoptionWire {
    async fn receive_offer_result(&mut self) -> anyhow::Result<crate::pty_custody::CustodyOffer> {
        let peer = self.peer.as_mut().context(FINALIZING_RETRY_PEER_CONTEXT)?;
        let offer: crate::pty_custody::CustodyOffer = peer.read().await?;
        anyhow::ensure!(
            offer.matches_request(&self.request),
            FINALIZING_RETRY_OFFER_DETAIL
        );
        self.received_descriptors = peer
            .receive_rights(offer.descriptor_set.descriptor_count())
            .await?;
        anyhow::ensure!(
            self.received_descriptors.len() == offer.descriptor_set.descriptor_count(),
            FINALIZING_RETRY_DESCRIPTOR_DETAIL
        );
        Ok(offer)
    }
}

#[cfg(target_os = "macos")]
const RETRY_PREDECESSOR_OVERFLOW_DETAIL: &str = "finalizing retry predecessor identity overflow";

#[cfg(target_os = "macos")]
impl SuccessorAdoptionWire {
    fn install_after_successor_loss(fixture: &Fixture) -> Self {
        let record = lock_recover(&fixture.state.record).clone();
        assert!(lock_recover(&fixture.state.secondary_custody).is_some());
        let active = fixture
            .state
            .secondary_custody_authority
            .active_for_test()
            .unwrap();
        let predecessor_holder = crate::pty_custody::CustodyHolderIdentity {
            pid: active.holder().pid,
            start_time: active
                .holder()
                .start_time
                .checked_add(CUSTODY_SUCCESSOR_INCREMENT)
                .expect(RETRY_PREDECESSOR_OVERFLOW_DETAIL),
        };
        let draft_predecessor = active.with_holder_for_test(predecessor_holder);
        let gone = crate::process::install_test_process_identity_relation(
            predecessor_holder.pid,
            predecessor_holder.start_time,
            crate::process::ProcessIdentityRelation::Gone,
        );
        let durable = armed_custody_with_identity_for_test(&record, draft_predecessor);
        let predecessor =
            secondary_custody_authority::ReadyWorkerCustodyIdentity::from_durable_for_test(
                durable.armed_lease().unwrap(),
            );
        fixture
            .state
            .secondary_custody_authority
            .replace_ready_for_test(predecessor);
        fixture
            .state
            .namespace
            .write_json(
                &fixture.state.paths.pty_custody(record.terminal_id),
                &durable,
            )
            .unwrap();
        let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
        let generation = record
            .controller_generation
            .saturating_add(CUSTODY_SUCCESSOR_INCREMENT);
        let (worker, peer) = UnixStream::pair().unwrap();
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

#[cfg(target_os = "macos")]
const FINALIZING_RETRY_PEER_CONTEXT: &str = "finalizing retry custody peer unavailable";
#[cfg(target_os = "macos")]
const FINALIZING_RETRY_OFFER_DETAIL: &str = "finalizing retry custody offer mismatch";
#[cfg(target_os = "macos")]
const FINALIZING_RETRY_DESCRIPTOR_DETAIL: &str = "finalizing retry custody descriptors mismatch";
#[cfg(target_os = "macos")]
const FINALIZING_ATTACHMENT_CLOSE_DETAIL: &str =
    "finalizing attachment connection did not close after its client was released";
