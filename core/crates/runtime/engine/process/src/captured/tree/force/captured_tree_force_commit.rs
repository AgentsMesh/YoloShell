#[cfg(any(test, feature = "runtime-component-tests"))]
use super::KillCommitPoll;
use super::{
    FrozenKillBasis, FrozenProcessTree, GoneConfirmation, KillCommitReady, ProcessClosureVersion,
};
#[cfg(any(test, feature = "runtime-component-tests"))]
use crate::process::captured_tree::transition::ProcessTransitionError;
use crate::process::captured_tree::{freeze, gone};

const OUTPUT_FENCE_VERSION_MISMATCH_DETAIL: &str =
    "output fence belongs to another process-tree closure";
impl FrozenProcessTree {
    pub fn closure_version(&self) -> ProcessClosureVersion {
        self.version.clone()
    }

    pub fn prepare_kill(
        &mut self,
        fenced_version: ProcessClosureVersion,
    ) -> anyhow::Result<KillCommitReady> {
        self.prepare_kill_with_destructive_commit(fenced_version, || Ok(()), || {})
    }

    pub(in crate::process) fn prepare_kill_with_destructive_commit(
        &mut self,
        fenced_version: ProcessClosureVersion,
        mut commit_fence: impl FnMut() -> anyhow::Result<()>,
        mut on_destructive_commit: impl FnMut(),
    ) -> anyhow::Result<KillCommitReady> {
        validate_output_fence_version(&self.version, &fenced_version)?;
        match self.kill_basis {
            FrozenKillBasis::ExactPositive | FrozenKillBasis::IncrementalExactPositive => {
                self.prepare_exact_kill_transaction(&mut commit_fence, &mut on_destructive_commit)
            }
            FrozenKillBasis::Ordinary => self
                .prepare_ordinary_kill_transaction(&mut commit_fence, &mut on_destructive_commit),
            FrozenKillBasis::KernelEof => Ok(KillCommitReady::Refreeze),
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn kill_after_output_fence(
        mut self,
        fenced_version: ProcessClosureVersion,
    ) -> Result<KillCommitPoll, ProcessTransitionError<Self>> {
        let ready = match self.prepare_kill(fenced_version) {
            Ok(ready) => ready,
            Err(error) => return Err(ProcessTransitionError::new(self, error)),
        };
        Ok(self.commit_kill(ready))
    }
}

pub(super) fn closure_remains_frozen(tree: &mut FrozenProcessTree) -> anyhow::Result<bool> {
    let first = freeze::inspect_fresh(&mut tree.tree)?;
    let second = freeze::inspect_fresh(&mut tree.tree)?;
    Ok(typed_closure_observations_match(
        &tree.closure,
        tree.kill_basis,
        first.as_ref(),
        second.as_ref(),
    ))
}

pub(super) fn typed_closure_observations_match(
    expected: &[crate::process::LiveProcessIdentity],
    basis: FrozenKillBasis,
    first: Option<&freeze::FreezeClosureCandidate>,
    second: Option<&freeze::FreezeClosureCandidate>,
) -> bool {
    let (Some(first), Some(second)) = (first, second) else {
        return false;
    };
    first == second
        && compatible_observation_closure(basis, Some(first))
            .is_some_and(|closure| closure == expected)
}

fn compatible_observation_closure(
    basis: FrozenKillBasis,
    candidate: Option<&freeze::FreezeClosureCandidate>,
) -> Option<&[crate::process::LiveProcessIdentity]> {
    match (basis, candidate?) {
        (FrozenKillBasis::ExactPositive, freeze::FreezeClosureCandidate::Exact(closure))
        | (
            FrozenKillBasis::IncrementalExactPositive,
            freeze::FreezeClosureCandidate::Exact(closure)
            | freeze::FreezeClosureCandidate::IncrementalExact(closure),
        )
        | (
            FrozenKillBasis::Ordinary,
            freeze::FreezeClosureCandidate::Exact(closure)
            | freeze::FreezeClosureCandidate::IncrementalExact(closure)
            | freeze::FreezeClosureCandidate::Ordinary(closure),
        )
        | (FrozenKillBasis::KernelEof, freeze::FreezeClosureCandidate::KernelEof(closure)) => {
            Some(closure)
        }
        _ => None,
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn closure_observations_match(
    expected: &[crate::process::LiveProcessIdentity],
    first: Option<&[crate::process::LiveProcessIdentity]>,
    second: Option<&[crate::process::LiveProcessIdentity]>,
) -> bool {
    first == Some(expected) && second == Some(expected)
}

pub(super) fn validate_output_fence_version(
    current: &ProcessClosureVersion,
    fenced: &ProcessClosureVersion,
) -> anyhow::Result<()> {
    anyhow::ensure!(fenced == current, OUTPUT_FENCE_VERSION_MISMATCH_DETAIL);
    Ok(())
}

pub(super) fn advance_gone_confirmation(
    previous: &GoneConfirmation,
    observation: gone::GoneObservation,
) -> (GoneConfirmation, bool) {
    match (previous, observation) {
        (GoneConfirmation::ObservedEmpty, gone::GoneObservation::Empty) => {
            (GoneConfirmation::Unconfirmed, true)
        }
        (_, gone::GoneObservation::Empty) => (GoneConfirmation::ObservedEmpty, false),
        _ => (GoneConfirmation::Unconfirmed, false),
    }
}
