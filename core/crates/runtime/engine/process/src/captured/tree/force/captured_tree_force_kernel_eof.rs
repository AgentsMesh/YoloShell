use super::{
    FrozenKillBasis, FrozenProcessTree, KernelEofKillCommitReady, KernelEofKillPreparation,
    KernelEofKillReady, KernelEofReopenSealCommit, KillCommitReady, ProcessClosureVersion,
};
use crate::process::captured_tree::{freeze, termination};
use crate::process::kernel_eof_authority::ValidatedSealedKernelEof;

const NONEMPTY_KERNEL_EOF_CLOSURE_DETAIL: &str =
    "kernel EOF kill authority requires a nonempty exact process closure";
const KERNEL_EOF_BASIS_REQUIRED_DETAIL: &str =
    "kernel EOF kill transaction requires a kernel EOF frozen basis";
const KERNEL_EOF_AUTHORITY_MISMATCH_DETAIL: &str =
    "kernel EOF validation belongs to another frozen transaction";
impl FrozenProcessTree {
    pub fn prepare_kill_with_kernel_eof(
        &mut self,
        fenced_version: ProcessClosureVersion,
    ) -> anyhow::Result<KernelEofKillPreparation> {
        super::commit::validate_output_fence_version(&self.version, &fenced_version)?;
        if self.kill_basis != FrozenKillBasis::KernelEof || self.refreeze_required {
            return Ok(KernelEofKillPreparation::Refreeze);
        }
        anyhow::ensure!(!self.closure.is_empty(), NONEMPTY_KERNEL_EOF_CLOSURE_DETAIL);
        if !closure_remains_frozen(self)? {
            self.refreeze_required = true;
            return Ok(KernelEofKillPreparation::Refreeze);
        }
        let seal = match self.prepare_kernel_eof_reopen_seal()? {
            KernelEofReopenSealCommit::FreshEofRequired => {
                return Ok(KernelEofKillPreparation::FreshEofRequired);
            }
            KernelEofReopenSealCommit::Reusable(seal) => seal,
        };
        if !closure_remains_frozen(self)? {
            self.refreeze_required = true;
            return Ok(KernelEofKillPreparation::Refreeze);
        }
        Ok(KernelEofKillPreparation::Ready(KernelEofKillReady {
            version: fenced_version,
            seal,
        }))
    }

    pub fn commit_kill_with_kernel_eof(
        &mut self,
        ready: KernelEofKillReady,
        validated: ValidatedSealedKernelEof,
    ) -> anyhow::Result<KillCommitReady> {
        super::commit::validate_output_fence_version(&self.version, &ready.version)?;
        anyhow::ensure!(
            self.kill_basis == FrozenKillBasis::KernelEof,
            KERNEL_EOF_BASIS_REQUIRED_DETAIL
        );
        anyhow::ensure!(
            self.kernel_eof
                .as_ref()
                .is_some_and(|authority| authority.consume_sealed(&ready.seal, validated)),
            KERNEL_EOF_AUTHORITY_MISMATCH_DETAIL
        );
        self.revalidate_kernel_eof_reopen_seal(&ready.seal)?;
        self.destructive_started = true;
        termination::signal(&self.tree)?;
        Ok(KillCommitReady::PendingKernelEof(
            KernelEofKillCommitReady {
                version: self.version.clone(),
            },
        ))
    }
}

pub(super) fn closure_remains_frozen(tree: &mut FrozenProcessTree) -> anyhow::Result<bool> {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    if let Some(stable) = tree.tree.test_state.kernel_eof_closure_stable_override() {
        return Ok(stable);
    }
    let first = inspect_kernel_eof_for_revalidation(tree)?;
    let second = inspect_kernel_eof_for_revalidation(tree)?;
    Ok(super::commit::typed_closure_observations_match(
        &tree.closure,
        tree.kill_basis,
        first.as_ref(),
        second.as_ref(),
    ))
}

fn inspect_kernel_eof_for_revalidation(
    tree: &mut FrozenProcessTree,
) -> anyhow::Result<Option<freeze::FreezeClosureCandidate>> {
    let root_exit_is_confirmed = tree.tree.root_exit_is_proven();
    let root_only_closure = tree
        .closure
        .iter()
        .all(|identity| identity == &tree.tree.live_root());
    if root_exit_is_confirmed && root_only_closure {
        freeze::inspect_with_kernel_eof(&mut tree.tree)
    } else {
        freeze::inspect_with_kernel_eof_fresh(&mut tree.tree)
    }
}

#[path = "captured_tree_force_kernel_eof_seal.rs"]
mod seal;
