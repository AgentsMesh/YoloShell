#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in super::super) enum DiscoveryScanEpoch {
    #[default]
    Inactive,
    Pending,
    Covered,
    Complete,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in super::super) struct DiscoveryEpochProgress {
    coverage_complete: bool,
    settlement_complete: bool,
    authoritative_empty: bool,
}

impl DiscoveryEpochProgress {
    pub(in super::super) fn coverage_only(authoritative_empty: bool) -> Self {
        Self {
            coverage_complete: true,
            settlement_complete: false,
            authoritative_empty,
        }
    }

    pub(in super::super) fn covered(capability_pending: bool, authoritative_empty: bool) -> Self {
        Self {
            coverage_complete: true,
            settlement_complete: !capability_pending,
            authoritative_empty,
        }
    }

    pub(in super::super) fn authoritative_empty(self) -> bool {
        self.authoritative_empty
    }
}

impl DiscoveryScanEpoch {
    pub(in super::super) fn requested(self) -> bool {
        matches!(self, Self::Pending | Self::Covered)
    }

    pub(in super::super) fn observe(&mut self, progress: DiscoveryEpochProgress) -> bool {
        if matches!(*self, Self::Inactive | Self::Complete) {
            return false;
        }
        *self = match progress {
            DiscoveryEpochProgress {
                coverage_complete: false,
                ..
            } => Self::Pending,
            DiscoveryEpochProgress {
                settlement_complete: true,
                ..
            } => Self::Complete,
            _ => Self::Covered,
        };
        true
    }

    pub(in super::super) fn coverage_is_complete(self) -> bool {
        matches!(self, Self::Covered | Self::Complete)
    }
}
