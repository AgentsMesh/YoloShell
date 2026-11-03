use super::{
    FreezingProcessTree, GoneConfirmation, GonePendingReady, KillGoneAuthority, KillGoneReady,
    KillPendingProcessTree, KillPoll, KillPollReady,
};
use crate::process::captured_tree::gone;
use crate::process::captured_tree::transition::ProcessTransitionError;

impl KillPendingProcessTree {
    pub fn prepare_poll(&mut self) -> anyhow::Result<KillPollReady> {
        match self.post_kill.observe(&mut self.tree)? {
            super::post_kill::PostKillObservation::Pending => {
                self.confirmation = GoneConfirmation::Unconfirmed;
                return Ok(KillPollReady::Pending(GonePendingReady(
                    GoneConfirmation::Unconfirmed,
                )));
            }
            super::post_kill::PostKillObservation::Refreeze => {
                return Ok(KillPollReady::Refreeze);
            }
            super::post_kill::PostKillObservation::ExactEmpty => {
                return Ok(self.confirm_empty(KillGoneAuthority::ExactContainment));
            }
            super::post_kill::PostKillObservation::Ordinary => {}
        }
        let previous = std::mem::replace(&mut self.confirmation, GoneConfirmation::Unconfirmed);
        let observation = gone::observe(&mut self.tree)?;
        if matches!(
            observation,
            gone::GoneObservation::NonEmpty
                | gone::GoneObservation::CapabilityScanPending
                | gone::GoneObservation::OutputFencePending
        ) {
            return Ok(KillPollReady::Refreeze);
        }
        let (confirmation, complete) =
            super::commit::advance_gone_confirmation(&previous, observation);
        #[cfg(target_os = "linux")]
        if complete
            && self.tree.linux_containment_available()
            && !self.tree.linux_containment_ready_for_gone()?
        {
            return Ok(KillPollReady::Refreeze);
        }
        Ok(if complete {
            KillPollReady::Gone(KillGoneReady(KillGoneAuthority::GlobalExclusion))
        } else {
            KillPollReady::Pending(GonePendingReady(confirmation))
        })
    }

    fn confirm_empty(&mut self, authority: KillGoneAuthority) -> KillPollReady {
        let previous = std::mem::replace(&mut self.confirmation, GoneConfirmation::Unconfirmed);
        let (confirmation, complete) =
            super::commit::advance_gone_confirmation(&previous, gone::GoneObservation::Empty);
        if complete {
            KillPollReady::Gone(KillGoneReady(authority))
        } else {
            KillPollReady::Pending(GonePendingReady(confirmation))
        }
    }

    pub fn commit_poll(mut self, ready: KillPollReady) -> KillPoll {
        match ready {
            KillPollReady::Refreeze => KillPoll::Refreeze(FreezingProcessTree::enter(
                self.tree,
                self.version,
                self.destructive_started,
            )),
            KillPollReady::Pending(GonePendingReady(confirmation)) => {
                self.confirmation = confirmation;
                KillPoll::Pending(self)
            }
            KillPollReady::Gone(KillGoneReady(authority)) => {
                if matches!(authority, KillGoneAuthority::GlobalExclusion) {
                    self.tree
                        .commit_confirmed_bootstrap_global_exclusion_if_authorized();
                }
                KillPoll::Gone(gone::complete(self.tree))
            }
        }
    }

    pub fn poll(mut self) -> Result<KillPoll, ProcessTransitionError<Self>> {
        let ready = match self.prepare_poll() {
            Ok(ready) => ready,
            Err(error) => return Err(ProcessTransitionError::new(self, error)),
        };
        Ok(self.commit_poll(ready))
    }
}
