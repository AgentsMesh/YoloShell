#[cfg(target_os = "macos")]
impl SuccessorAdoptionWire {
    async fn receive_commit(
        &mut self,
        offer: &crate::pty_custody::CustodyOffer,
    ) -> crate::pty_custody::CustodyWorkerCommit {
        let acknowledgement = match offer.profile {
            crate::pty_custody::CustodyProfile::ReleasedTerminal => {
                crate::pty_custody::CustodyHostAck::prepared(offer)
            }
            crate::pty_custody::CustodyProfile::ActiveRunning
            | crate::pty_custody::CustodyProfile::ActiveTerminalReadOnly => {
                crate::pty_custody::CustodyHostAck::prepared_guarded(
                    offer,
                    self.prepare_guardian(offer),
                )
            }
        };
        let peer = self.peer.as_mut().unwrap();
        peer.write(&acknowledgement).await.unwrap();
        let commit: crate::pty_custody::CustodyWorkerCommit = peer.read().await.unwrap();
        assert!(commit.matches_offer(offer));
        commit
    }

    fn prepare_guardian(
        &self,
        offer: &crate::pty_custody::CustodyOffer,
    ) -> crate::pty_custody::CustodyGuardianPreparation {
        let path = self
            .state
            .paths
            .pty_custody(self.request.runtime.terminal_id);
        let runtime_path = self
            .state
            .paths
            .runtime_record(self.request.runtime.terminal_id);
        let (_, runtime_identity): (crate::model::RuntimeRecord, _) = self
            .state
            .namespace
            .read_json_with_identity(&runtime_path)
            .unwrap();
        let (predecessor, predecessor_identity): (crate::pty_custody::CustodyDurableRecord, _) =
            self.state.namespace.read_json_with_identity(&path).unwrap();
        let guardian = crate::pty_custody::CustodyGuardianBinding::new(
            uuid::Uuid::new_v4(),
            self.request.holder(),
            &self.request,
            offer,
        )
        .unwrap();
        let prepared = predecessor
            .prepare_guardian(&self.request, offer, guardian)
            .unwrap();
        let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &self.state.paths,
            self.state.namespace.lease(),
            self.request.runtime.terminal_id,
        )
        .unwrap();
        let custody_identity = crate::pty_custody::replace_exact_record(
            &guard,
            &path,
            &predecessor,
            predecessor_identity,
            &prepared,
        )
        .unwrap();
        crate::pty_custody::CustodyGuardianPreparation::new(
            guardian,
            runtime_identity,
            predecessor_identity,
            custody_identity,
        )
    }
}
