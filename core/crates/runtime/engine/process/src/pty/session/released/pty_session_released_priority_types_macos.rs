use super::{ReleasedHolderHints, ReleasedHolderQueryObservation, ReleasedSweepPolicy};
use crate::process::LiveProcessIdentity;
use crate::process::pty_session_proof::macos_liveness::MacosGlobalScanMode;
use std::collections::HashSet;

pub(in crate::process::pty_session_proof::macos_released) struct ReleasedPrioritySelection {
    pub(in crate::process::pty_session_proof::macos_released) identities:
        HashSet<LiveProcessIdentity>,
    pub(in crate::process::pty_session_proof::macos_released) holder_identities:
        HashSet<LiveProcessIdentity>,
    pub(in crate::process::pty_session_proof::macos_released) observed_holder_identities:
        HashSet<LiveProcessIdentity>,
    pub(in crate::process::pty_session_proof::macos_released) holder_hints: ReleasedHolderHints,
    pub(in crate::process::pty_session_proof::macos_released) observation:
        ReleasedHolderQueryObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::process::pty_session_proof::macos_released) enum ReleasedHolderQueryPolicy {
    Refresh,
    RetainKnown,
}

impl ReleasedHolderQueryPolicy {
    pub(in crate::process::pty_session_proof::macos_released) fn for_sweep(
        policy: ReleasedSweepPolicy,
        mode: MacosGlobalScanMode,
    ) -> Self {
        if matches!(
            mode,
            MacosGlobalScanMode::CustodyPtyObject | MacosGlobalScanMode::RecoveryCustodyPtyObject
        ) {
            return Self::Refresh;
        }
        match policy {
            ReleasedSweepPolicy::Live => Self::Refresh,
            ReleasedSweepPolicy::Frozen => Self::RetainKnown,
        }
    }
}

pub(in crate::process::pty_session_proof::macos_released) fn selection(
    identities: HashSet<LiveProcessIdentity>,
    holder_identities: HashSet<LiveProcessIdentity>,
    observed_holder_identities: HashSet<LiveProcessIdentity>,
    holder_hints: ReleasedHolderHints,
    observation: ReleasedHolderQueryObservation,
) -> ReleasedPrioritySelection {
    ReleasedPrioritySelection {
        identities,
        holder_identities,
        observed_holder_identities,
        holder_hints,
        observation,
    }
}
