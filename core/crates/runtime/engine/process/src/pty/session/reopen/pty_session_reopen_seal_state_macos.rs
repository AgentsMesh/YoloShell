use super::macos_reopen_seal::MacosPtyReopenSeal;

#[derive(Default)]
pub(super) enum MacosPtyReopenSealState {
    #[default]
    Idle,
    Active(MacosPtyReopenSeal),
    Quarantined,
    Completed,
}

impl MacosPtyReopenSealState {
    pub(super) fn active(&self) -> Option<&MacosPtyReopenSeal> {
        match self {
            Self::Active(seal) => Some(seal),
            Self::Idle | Self::Quarantined | Self::Completed => None,
        }
    }

    pub(super) fn active_mut(&mut self) -> Option<&mut MacosPtyReopenSeal> {
        match self {
            Self::Active(seal) => Some(seal),
            Self::Idle | Self::Quarantined | Self::Completed => None,
        }
    }

    pub(super) fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub(super) fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }
}
