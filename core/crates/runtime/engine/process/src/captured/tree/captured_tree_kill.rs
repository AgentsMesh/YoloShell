use super::{CapturedProcessTree, LiveProcessIdentity, ProcessClosureVersion};
use crate::process::captured_targets;
use anyhow::Result;

pub struct ExactClosureKill {
    closure: Vec<LiveProcessIdentity>,
    version: ProcessClosureVersion,
    authority: ExactKillAuthority,
    _resolution: captured_targets::ExactSigkillResolution,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum ExactKillAuthority {
    ExactPositive,
    IncrementalExactPositive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RetainedExclusion {
    Ready,
    Pending,
    Revoked,
}

impl ExactClosureKill {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(super) fn for_test(
        targets: Vec<(LiveProcessIdentity, crate::process::ProcessIdentity)>,
        version: ProcessClosureVersion,
        authority: ExactKillAuthority,
    ) -> Self {
        let closure = targets
            .iter()
            .map(|(identity, _durable)| identity.clone())
            .collect();
        Self {
            closure,
            version,
            authority,
            _resolution: captured_targets::ExactSigkillResolution::from_targets(&targets),
        }
    }

    pub(super) fn matches(
        &self,
        closure: &[LiveProcessIdentity],
        version: &ProcessClosureVersion,
        authority: ExactKillAuthority,
    ) -> bool {
        self.closure == closure && &self.version == version && self.authority == authority
    }

    pub(super) fn closure(&self) -> &[LiveProcessIdentity] {
        &self.closure
    }

    pub(super) fn version(&self) -> &ProcessClosureVersion {
        &self.version
    }

    pub(super) fn retained_exclusion(&self) -> RetainedExclusion {
        match self.authority {
            ExactKillAuthority::ExactPositive => RetainedExclusion::Pending,
            ExactKillAuthority::IncrementalExactPositive => RetainedExclusion::Revoked,
        }
    }
}

#[cfg(target_os = "macos")]
impl RetainedExclusion {
    pub(super) fn observe(self, tree: &mut CapturedProcessTree) -> Self {
        if self == Self::Revoked || retained_exclusion_is_revoked(tree) {
            return Self::Revoked;
        }
        if !tree.lineage_settled
            || tree.lineage_integrity_lost
            || !tree.events.inventory_publication_is_current()
        {
            return Self::Pending;
        }
        let quiescent = tree.events.post_inventory_quiescent();
        if retained_exclusion_is_revoked(tree) {
            Self::Revoked
        } else if quiescent {
            Self::Ready
        } else {
            Self::Pending
        }
    }
}

#[cfg(target_os = "macos")]
fn retained_exclusion_is_revoked(tree: &CapturedProcessTree) -> bool {
    !tree.events.is_gated()
        || tree.frozen_fork_reconciliation_required
        || tree.events.has_unresolved_fork()
        || !tree
            .pty_proof
            .as_ref()
            .is_some_and(super::PtySessionProof::has_transferable_descriptor)
}

#[cfg(not(target_os = "macos"))]
impl RetainedExclusion {
    pub(super) fn observe(self, _tree: &mut CapturedProcessTree) -> Self {
        Self::Revoked
    }
}

pub(super) fn signal(tree: &CapturedProcessTree) -> Result<()> {
    let targets = tree.captured_targets();
    let authority = tree.destructive_authority()?;
    captured_targets::signal(authority, &targets, libc::SIGKILL)?;
    Ok(())
}

pub(super) fn signal_exact(
    tree: &CapturedProcessTree,
    expected: &[LiveProcessIdentity],
    version: ProcessClosureVersion,
    exact_authority: ExactKillAuthority,
) -> Result<Option<ExactClosureKill>> {
    let targets = tree.captured_targets();
    if !targets
        .iter()
        .map(|(identity, _durable)| identity)
        .eq(expected)
    {
        return Ok(None);
    }
    let authority = tree.destructive_authority()?;
    Ok(
        captured_targets::signal_sigkill(authority, &targets)?.map(|resolution| ExactClosureKill {
            closure: expected.to_vec(),
            version,
            authority: exact_authority,
            _resolution: resolution,
        }),
    )
}
