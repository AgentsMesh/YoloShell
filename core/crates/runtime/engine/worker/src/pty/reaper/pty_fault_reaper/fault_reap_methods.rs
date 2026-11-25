impl FaultReap {
    pub(crate) fn finish_reaped(&mut self, proof: crate::process::UnreapedRootExit) -> bool {
        let Some(authority) = self.custody_mut().authority.take() else {
            return false;
        };
        match authority.reap_preserving_authority(proof) {
            Ok(status) => {
                self.custody_mut().processes = None;
                #[cfg(test)]
                let failure_present = self.failure.is_some();
                self.publish_terminal_status();
                let mut outcome = ReapOutcome::waited(Some(status));
                outcome.fault = self.failure.take();
                self.completion().publish(outcome);
                #[cfg(test)]
                self.publish_terminal_state_for_test(failure_present);
                true
            }
            Err(failure) => {
                let loss = supervision::contain(|| WaitAuthorityLoss::from_error(failure.error()))
                    .flatten();
                let (authority, error) = failure.into_parts();
                self.remember_terminal(error);
                if let Some(loss) = loss {
                    self.lost_wait_custody = Some(authority.lose(loss));
                } else {
                    self.custody_mut().authority = Some(authority);
                    self.custody_mut().root_exit_proof = Some(proof);
                }
                false
            }
        }
    }

    pub(crate) fn finish_lost(&mut self) {
        self.custody_mut().processes = None;
        self.custody_mut().authority = None;
        #[cfg(test)]
        let failure_present = self.failure.is_some();
        let mut outcome = ReapOutcome::lost_unknown();
        self.lost_wait_custody = None;
        self.publish_terminal_status();
        outcome.fault = self.failure.take();
        self.completion().publish(outcome);
        #[cfg(test)]
        self.publish_terminal_state_for_test(failure_present);
    }

    pub(crate) fn remember(&mut self, result: Result<()>) {
        let Err(error) = result else {
            return;
        };
        if self.completion().is_published() {
            self.publish_fault(&error);
            terminal_reporting::warn_error(error, constants::POST_COMPLETION_FAULT_DETAIL);
        } else if self.failure.is_none() {
            self.publish_fault(&error);
            self.failure = Some(error);
        } else {
            terminal_reporting::discard_error(error);
        }
    }

    pub(crate) fn remember_terminal(&mut self, terminal: anyhow::Error) {
        if self.completion().is_published() {
            self.remember(Err(terminal));
        } else {
            failure::remember_terminal(&mut self.failure, terminal);
            if let Some(failure) = self.failure.as_ref() {
                self.publish_fault(failure);
            }
        }
    }

    pub(crate) fn lose_wait_authority(&mut self, proof: WaitAuthorityLoss) {
        if let Some(authority) = self.custody_mut().authority.take() {
            self.lost_wait_custody = Some(authority.lose(proof));
        }
    }

    pub(crate) fn has_unsettled_custody(&self) -> bool {
        let custody = self.custody();
        custody.processes.is_some()
            || custody.authority.is_some()
            || self.lost_wait_custody.is_some()
    }
}
