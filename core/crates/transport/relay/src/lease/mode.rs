#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LeaseAcquireMode {
    Disabled,
    Observe,
    Takeover,
}

impl LeaseAcquireMode {
    pub(super) const fn enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub(super) const fn retaining_intent(self, retry: bool) -> Self {
        if !retry {
            return Self::Disabled;
        }
        match self {
            Self::Disabled => Self::Observe,
            mode => mode,
        }
    }
}
