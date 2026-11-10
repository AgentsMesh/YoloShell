use super::{OutputFence, PtyForceTermination};
use crate::worker::pty_output_progress::{ExclusiveOutputQuiescenceFence, OutputQuiescenceFence};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ForceTransactionState {
    Unstarted,
    Started,
}

#[derive(Eq, PartialEq)]
pub(crate) enum OutputFenceProgress {
    Reader(std::mem::Discriminant<OutputQuiescenceFence>),
    Exclusive(std::mem::Discriminant<ExclusiveOutputQuiescenceFence>),
    #[cfg(test)]
    Scripted,
    Reached,
}

impl PtyForceTermination {
    pub(crate) fn record_force_request(&mut self, force: bool) {
        self.force_requested |= force;
    }

    pub(crate) fn allows_graceful_fault_handoff(&self) -> bool {
        !self.force_requested && self.transaction_state == ForceTransactionState::Unstarted
    }

    pub(crate) fn force_was_requested(&self) -> bool {
        self.force_requested
    }

    pub(crate) fn begin_transaction(&mut self) {
        self.transaction_state = ForceTransactionState::Started;
    }

    pub(crate) fn transaction_state(&self) -> ForceTransactionState {
        self.transaction_state
    }

    pub(crate) fn mark_progress(&mut self) {
        self.progressed = true;
    }

    pub(crate) fn take_progress(&mut self) -> bool {
        std::mem::take(&mut self.progressed)
    }

    #[cfg(test)]
    pub(crate) fn output_fence_reached_for_test(&self) -> Option<bool> {
        self.output_fence
            .as_ref()
            .map(|fence| matches!(fence.state, OutputFence::Reached))
    }
}

impl OutputFence {
    pub(crate) fn progress(&self) -> OutputFenceProgress {
        match self {
            Self::Reader(fence) => OutputFenceProgress::Reader(std::mem::discriminant(fence)),
            Self::Exclusive(fence) => OutputFenceProgress::Exclusive(std::mem::discriminant(fence)),
            #[cfg(test)]
            Self::Scripted => OutputFenceProgress::Scripted,
            Self::Reached => OutputFenceProgress::Reached,
        }
    }
}
