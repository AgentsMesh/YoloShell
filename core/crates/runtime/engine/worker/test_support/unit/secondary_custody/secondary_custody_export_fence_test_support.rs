#[cfg(target_os = "macos")]
const EXPORT_FENCE_EXPECTED_RELEASE_DETAIL: &str =
    "fenced successor adoption unexpectedly emitted activation";
#[cfg(target_os = "macos")]
const SINGLE_ACTIVE_EXPORT: (usize, usize) = (1, 0);
#[cfg(target_os = "macos")]
const SINGLE_ORPHAN_EXPORT: (usize, usize) = (0, 1);
#[cfg(target_os = "macos")]
const NO_SUCCESSOR_EXPORTS: (usize, usize) = (0, 0);

#[cfg(target_os = "macos")]
impl SuccessorAdoptionWire {
    fn connect_to_installed_source(fixture: &Fixture) -> Self {
        let record = lock_recover(&fixture.state.record).clone();
        let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
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
            pair: None,
            writer: None,
            _gone: None,
        }
    }

    async fn receive_pre_activation_release(
        &mut self,
        offer: &crate::pty_custody::CustodyOffer,
    ) -> crate::pty_custody::CustodyControl {
        let disposition: crate::pty_custody::CustodyWorkerReadyDisposition =
            self.peer.as_mut().unwrap().read().await.unwrap();
        let crate::pty_custody::CustodyWorkerReadyDisposition::Release { control } = disposition
        else {
            panic!("{EXPORT_FENCE_EXPECTED_RELEASE_DETAIL}");
        };
        assert!(control.matches_release(offer));
        control
    }

    async fn acknowledge_exact_release(&mut self, control: &crate::pty_custody::CustodyControl) {
        let acknowledgement = crate::pty_custody::CustodyControlAck::released(control).unwrap();
        self.peer
            .as_mut()
            .unwrap()
            .write(&acknowledgement)
            .await
            .unwrap();
    }

    fn drop_received_descriptors(&mut self) {
        self.received_descriptors.clear();
    }

    async fn assert_offer_not_emitted(&mut self) {
        let result = self
            .peer
            .as_mut()
            .unwrap()
            .read::<crate::pty_custody::CustodyOffer>()
            .await;
        assert!(result.is_err());
        assert!(self.received_descriptors.is_empty());
    }
}

#[cfg(target_os = "macos")]
async fn poll_export_fenced_alias_closure(
    fixture: &Fixture,
) -> pty_runtime_alias_custody::PtyRuntimeAliasClosePoll {
    let permit = fixture.state.admit_system_mutation().await;
    let closure = fixture.state.prepare_system_alias_release(&permit);
    fixture.state.commit_system_alias_release(&permit, closure);
    drop(permit);
    fixture
        .state
        .pty_alias_custody
        .close_local_aliases()
        .unwrap()
}
