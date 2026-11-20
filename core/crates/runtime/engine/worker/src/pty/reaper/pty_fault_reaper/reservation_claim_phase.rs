use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum ClaimPhase {
    Vacant,
    Installing,
    Installed,
    Ready,
    Cancelled,
    Settled,
}

pub(crate) struct ClaimPhaseState {
    pub(crate) phase: AtomicU8,
}

impl ClaimPhaseState {
    pub(crate) fn new(phase: ClaimPhase) -> Self {
        Self {
            phase: AtomicU8::new(phase as u8),
        }
    }

    pub(crate) fn load(&self) -> ClaimPhase {
        match self.phase.load(Ordering::Acquire) {
            value if value == ClaimPhase::Vacant as u8 => ClaimPhase::Vacant,
            value if value == ClaimPhase::Installing as u8 => ClaimPhase::Installing,
            value if value == ClaimPhase::Installed as u8 => ClaimPhase::Installed,
            value if value == ClaimPhase::Ready as u8 => ClaimPhase::Ready,
            value if value == ClaimPhase::Cancelled as u8 => ClaimPhase::Cancelled,
            _ => ClaimPhase::Settled,
        }
    }

    pub(crate) fn begin_install(&self) -> bool {
        self.phase
            .compare_exchange(
                ClaimPhase::Vacant as u8,
                ClaimPhase::Installing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub(crate) fn mark_installed(&self) {
        self.phase
            .store(ClaimPhase::Installed as u8, Ordering::Release);
    }

    pub(crate) fn mark_ready(&self) {
        self.phase.store(ClaimPhase::Ready as u8, Ordering::Release);
    }

    pub(crate) fn cancel_vacant(&self) -> bool {
        self.phase
            .compare_exchange(
                ClaimPhase::Vacant as u8,
                ClaimPhase::Cancelled as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub(crate) fn mark_settled(&self) {
        self.phase
            .store(ClaimPhase::Settled as u8, Ordering::Release);
    }
}

impl ClaimPhase {
    pub(crate) fn is_waiting(self) -> bool {
        matches!(self, Self::Vacant | Self::Installing | Self::Installed)
    }
}
