use super::{
    MacosProcessEvents, NO_EVENTS, OBSERVED_NOTES, REGISTER_CONTEXT, current_identities,
    is_missing, process_event,
};
use crate::process::LiveProcessIdentity;
use anyhow::{Context, Result, ensure};

pub(super) enum RegistrationOutcome {
    Unchanged,
    Registered,
    CandidateRetired,
}

pub(in crate::process) struct ReconcileOutcome {
    pub(in crate::process) registered: Vec<LiveProcessIdentity>,
    pub(in crate::process) invalidated: bool,
}

impl MacosProcessEvents {
    pub(in crate::process) fn reconcile(
        &mut self,
        identities: impl IntoIterator<Item = LiveProcessIdentity>,
    ) -> Result<ReconcileOutcome> {
        let result = self.reconcile_current(identities);
        if result.is_err() {
            self.registered.clear();
            self.queue.take();
        }
        result
    }

    fn reconcile_current(
        &mut self,
        identities: impl IntoIterator<Item = LiveProcessIdentity>,
    ) -> Result<ReconcileOutcome> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        self.test_faults.check_reconcile()?;
        let selection = current_identities(identities)?;
        let mut expected = selection.current;
        let mut invalidated = selection.invalidated;
        invalidated |= self.changed()?;
        expected.retain(|_, identity| !self.exact_gone_fork_source(*identity));
        for (pid, identity) in self.registered.clone() {
            if expected.get(&pid) != Some(&identity) {
                if self.generation_can_still_fork(identity)? {
                    // An inventory omission is not proof that a live source vanished.
                    // Retain its watcher and force a fresh inventory publication.
                    invalidated = true;
                    continue;
                }
                #[cfg(any(test, feature = "runtime-component-tests"))]
                self.test_faults.queue_before_unregister_read();
                // The exact generation can no longer fork. Drain everything it could
                // have queued before that proof while its knote is still registered.
                invalidated |= self.changed()?;
                self.mark_generation_gone(identity);
                self.unregister(pid)?;
            }
        }
        let mut registered = Vec::new();
        for identity in expected.into_values() {
            match self.register_verified(identity)? {
                RegistrationOutcome::Unchanged => {}
                RegistrationOutcome::Registered => registered.push(identity),
                RegistrationOutcome::CandidateRetired => invalidated = true,
            }
        }
        Ok(ReconcileOutcome {
            registered,
            invalidated,
        })
    }

    pub(super) fn register_verified(
        &mut self,
        identity: LiveProcessIdentity,
    ) -> Result<RegistrationOutcome> {
        if self.exact_gone_fork_source(identity) {
            return Ok(RegistrationOutcome::Unchanged);
        }
        if !identity.try_matches()? {
            return Ok(RegistrationOutcome::CandidateRetired);
        }
        if self.registered.get(&identity.pid()) == Some(&identity) {
            return Ok(RegistrationOutcome::Unchanged);
        }
        let registration = self.change(process_event(
            identity.pid(),
            libc::EV_ADD | libc::EV_CLEAR,
            OBSERVED_NOTES,
        ));
        match registration {
            Err(error) if is_missing(&error) => {
                return Ok(RegistrationOutcome::CandidateRetired);
            }
            result => result?,
        }
        self.registered.insert(identity.pid(), identity);
        let still_current = identity.try_matches()?;
        if still_current {
            return Ok(RegistrationOutcome::Registered);
        }
        let _ = self.changed()?;
        self.mark_generation_gone(identity);
        self.unregister(identity.pid())?;
        Ok(RegistrationOutcome::CandidateRetired)
    }

    pub(super) fn unregister(&mut self, pid: u32) -> Result<()> {
        self.registered.remove(&pid);
        let result = self.change(process_event(pid, libc::EV_DELETE, NO_EVENTS as u32));
        match result {
            Err(error) if is_missing(&error) => Ok(()),
            result => result,
        }
    }

    fn change(&mut self, change: libc::kevent) -> Result<()> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        self.test_faults.before_change(&change)?;
        let result = self
            .submit(Some(&change), &mut [], None)
            .context(REGISTER_CONTEXT)?;
        ensure!(result == usize::default(), REGISTER_CONTEXT);
        #[cfg(any(test, feature = "runtime-component-tests"))]
        self.test_faults.after_change(&change);
        Ok(())
    }
}
