use crate::worker::pty_output_progress::KernelPtyEofIdentity;
use std::sync::Arc;

#[derive(Debug, Default)]
pub(crate) enum ReaderEofState {
    #[default]
    Open,
    AwaitingPublication(Arc<KernelPtyEofIdentity>),
    Retained(Arc<KernelPtyEofIdentity>),
    Observed(Option<Arc<KernelPtyEofIdentity>>),
}

pub(crate) enum KernelEofTransactionDisposition {
    Complete,
    Preserve,
    PreserveSealed(crate::worker::pty_output_progress::CommittedKernelEofSealEpoch),
    Revoke,
}

impl ReaderEofState {
    pub(crate) fn identity(&self) -> Option<&Arc<KernelPtyEofIdentity>> {
        match self {
            Self::AwaitingPublication(identity) | Self::Retained(identity) => Some(identity),
            Self::Observed(identity) => identity.as_ref(),
            Self::Open => None,
        }
    }

    pub(crate) fn is_current(&self, expected: &Arc<KernelPtyEofIdentity>) -> bool {
        self.identity()
            .is_some_and(|current| Arc::ptr_eq(current, expected))
    }

    pub(crate) fn is_awaiting_publication(&self) -> bool {
        matches!(self, Self::AwaitingPublication(_))
    }

    pub(crate) fn is_retained(&self) -> bool {
        matches!(self, Self::Retained(_))
    }

    pub(crate) fn is_observed(&self) -> bool {
        matches!(self, Self::Observed(_))
    }

    pub(crate) fn has_authority(&self) -> bool {
        self.identity().is_some()
    }

    pub(crate) fn await_publication(&mut self, identity: Arc<KernelPtyEofIdentity>) {
        *self = Self::AwaitingPublication(identity);
    }

    pub(crate) fn retain(&mut self, expected: &Arc<KernelPtyEofIdentity>) -> bool {
        if !matches!(self, Self::AwaitingPublication(_)) || !self.is_current(expected) {
            return false;
        }
        *self = Self::Retained(Arc::clone(expected));
        true
    }

    pub(crate) fn retain_observation(&mut self, identity: Arc<KernelPtyEofIdentity>) {
        *self = Self::Retained(identity);
    }

    pub(crate) fn publish(&mut self, expected: &Arc<KernelPtyEofIdentity>) -> bool {
        if !self.is_current(expected) {
            return false;
        }
        *self = Self::Observed(Some(Arc::clone(expected)));
        true
    }

    pub(crate) fn observe(&mut self, identity: Arc<KernelPtyEofIdentity>) {
        *self = Self::Observed(Some(identity));
    }

    pub(crate) fn mark_drained(&mut self) {
        *self = Self::Observed(None);
    }

    pub(crate) fn revoke_if_current(&mut self, expected: &Arc<KernelPtyEofIdentity>) -> bool {
        if !self.is_current(expected) {
            return false;
        }
        self.clear();
        true
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::Open;
    }
}
