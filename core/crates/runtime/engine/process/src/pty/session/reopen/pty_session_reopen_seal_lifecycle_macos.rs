impl PtySessionProof {
    pub(in crate::process) fn mark_reopen_seal_destructive_fence_crossed(&mut self) -> Result<()> {
        let marked = self.with_reopen_seal(|seal, _proof| seal.mark_destructive_fence_crossed())?;
        ensure!(
            marked.is_some() || self.is_process_discovery(),
            REOPEN_SEAL_MISSING_DETAIL
        );
        Ok(())
    }

    pub(in crate::process) fn reopen_seal_commit_requires_fresh_eof(&self) -> bool {
        self.reopen_seal
            .active()
            .is_some_and(super::macos_reopen_seal::MacosPtyReopenSeal::requires_fresh_eof)
    }

    pub(in crate::process) fn seal_reopen_before_master_alias_close(&mut self) -> Result<()> {
        let authority = self.acquire_reopen_seal()?;
        self.install_reopen_seal(authority)?;
        self.commit_reopen_seal_for_destruction()?;
        self.prepare_committed_reopen_seal_for_master_alias_close()
    }

    pub(in crate::process) fn commit_reopen_seal_for_destruction(&mut self) -> Result<()> {
        let committed = self.with_reopen_seal(|seal, proof| seal.commit(proof))?;
        ensure!(
            committed.is_some() || self.is_process_discovery(),
            REOPEN_SEAL_MISSING_DETAIL
        );
        Ok(())
    }

    pub(in crate::process) fn settle_reopen_seal_after_fence(&mut self) -> Result<()> {
        let settled = self.with_reopen_seal(|seal, proof| seal.settle_after_fence(proof))?;
        ensure!(
            settled.is_some() || self.is_process_discovery(),
            REOPEN_SEAL_MISSING_DETAIL
        );
        Ok(())
    }

    fn prepare_committed_reopen_seal_for_master_alias_close(&mut self) -> Result<()> {
        let prepared =
            self.with_reopen_seal(|seal, proof| seal.prepare_master_alias_close(proof))?;
        ensure!(
            prepared.is_some() || self.is_process_discovery(),
            REOPEN_SEAL_MISSING_DETAIL
        );
        Ok(())
    }

    pub(in crate::process) fn complete_reopen_seal_after_master_alias_close(
        &mut self,
    ) -> Result<()> {
        self.validate_reopen_seal_after_master_alias_close()?;
        self.reopen_seal = super::macos_reopen_seal_state::MacosPtyReopenSealState::Completed;
        Ok(())
    }

    pub(in crate::process) fn validate_reopen_seal_after_master_alias_close(&self) -> Result<()> {
        ensure!(
            !matches!(
                &self.master_alias_state,
                super::PtyMasterAliasState::Unbound
            ),
            REOPEN_SEAL_MISSING_DETAIL
        );
        match &self.reopen_seal {
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Active(seal) => {
                seal.validate_master_alias_close()?
            }
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Completed => {}
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Quarantined => {
                bail!(REOPEN_SEAL_MISSING_DETAIL)
            }
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Idle
                if self.is_process_discovery() => {}
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Idle => {
                bail!(REOPEN_SEAL_MISSING_DETAIL)
            }
        }
        Ok(())
    }

    pub(in crate::process) fn complete_reopen_seal_without_master_aliases(&mut self) -> Result<()> {
        if self.reopen_seal.is_completed() {
            return Ok(());
        }
        if self.is_process_discovery() {
            return self.complete_reopen_seal_after_master_alias_close();
        }
        self.release_no_alias_object_lease()?;
        self.complete_reopen_seal_after_master_alias_close()
    }

    pub(super) fn reopen_seal_prepared_for_master_alias_close(&self) -> bool {
        self.reopen_seal.is_completed()
            || self
                .reopen_seal
                .active()
                .is_some_and(|seal| seal.master_alias_close_is_prepared())
    }

    pub(in crate::process) fn rollback_reopen_seal_before_commit(&mut self) -> Result<()> {
        match &mut self.reopen_seal {
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Active(seal) => {
                seal.restore_before_commit()?;
                self.reopen_seal = Default::default();
            }
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Idle
            | super::macos_reopen_seal_state::MacosPtyReopenSealState::Quarantined
            | super::macos_reopen_seal_state::MacosPtyReopenSealState::Completed => {}
        }
        Ok(())
    }

    pub(in crate::process) fn quarantine_reopen_seal(&mut self) -> Result<()> {
        let previous = std::mem::replace(
            &mut self.reopen_seal,
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Idle,
        );
        let super::macos_reopen_seal_state::MacosPtyReopenSealState::Active(mut seal) = previous
        else {
            self.reopen_seal = previous;
            return Ok(());
        };
        if let Err(error) = seal.quarantine_fail_closed(self) {
            self.reopen_seal =
                super::macos_reopen_seal_state::MacosPtyReopenSealState::Active(seal);
            return Err(error);
        }
        self.reopen_seal = super::macos_reopen_seal_state::MacosPtyReopenSealState::Quarantined;
        Ok(())
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process) fn rollback_reopen_seal_before_commit_for_test(&mut self) -> Result<()> {
        self.rollback_reopen_seal_before_commit()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process) fn reopen_seal_is_quarantined_for_test(&self) -> bool {
        matches!(
            self.reopen_seal,
            super::macos_reopen_seal_state::MacosPtyReopenSealState::Quarantined
        )
    }
}
