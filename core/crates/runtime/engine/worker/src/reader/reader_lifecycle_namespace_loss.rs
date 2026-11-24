use super::ReaderLifecycle;
use crate::namespace_lease::NamespaceOwnershipLoss;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum NamespaceLossDrainState {
    #[default]
    Inactive,
    Active,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NamespaceLossDrainAdmission {
    Blocked,
    Entered,
    Active,
}

impl ReaderLifecycle {
    pub(crate) fn begin_namespace_loss_drain(
        &self,
        _proof: NamespaceOwnershipLoss,
    ) -> NamespaceLossDrainAdmission {
        self.enter_namespace_loss_drain()
    }

    pub(crate) fn drains_after_namespace_loss(&self) -> bool {
        self.lock_progress().namespace_loss_drain == NamespaceLossDrainState::Active
    }

    fn enter_namespace_loss_drain(&self) -> NamespaceLossDrainAdmission {
        let mut progress = self.lock_progress();
        if progress.output_read_in_flight || !progress.pending.is_empty() {
            return NamespaceLossDrainAdmission::Blocked;
        }
        progress.output_abandoned = true;
        progress.kernel_eof_probe_identity = None;
        self.output_reader_wakeup
            .store(true, std::sync::atomic::Ordering::Release);
        let admission = match progress.namespace_loss_drain {
            NamespaceLossDrainState::Inactive => NamespaceLossDrainAdmission::Entered,
            NamespaceLossDrainState::Active => NamespaceLossDrainAdmission::Active,
        };
        progress.namespace_loss_drain = NamespaceLossDrainState::Active;
        self.changed.notify_all();
        admission
    }
}
