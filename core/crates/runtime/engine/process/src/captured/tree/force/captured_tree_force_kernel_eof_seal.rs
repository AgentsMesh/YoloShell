use super::super::reopen_seal_cleanup::ReopenSealRecoveryIntent;
use super::super::reopen_seal_terminal::{consume_error_detail, consume_failure_detail};
use super::super::{FrozenProcessTree, KernelEofKillReady, KernelEofReopenSealCommit};
use crate::process::CommittedPtyReopenSealAuthority;
use crate::process::captured_tree::CapturedProcessTree;
use crate::process::pty_session_proof::PtyReopenSealAuthority;
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};

const REOPEN_SEAL_TRANSACTION_INVALIDATED_DETAIL: &str =
    "PTY reopen-seal transaction invalidated the observed kernel EOF";
const REOPEN_SEAL_CLEANUP_FAILURE_DETAIL: &str =
    "failed to resolve an interrupted PTY reopen-seal transaction";
const REOPEN_SEAL_SOURCE_FORMATTING_PANIC_DETAIL: &str =
    "PTY reopen-seal source error formatting panicked";
#[cfg(any(test, feature = "runtime-component-tests"))]
const TEST_REOPEN_SEAL_INVALIDATED_DETAIL: &str = "injected PTY reopen-seal output invalidation";

#[derive(Debug)]
struct KernelEofOutputInvalidated {
    source: String,
    cleanup: Option<String>,
}

impl std::fmt::Display for KernelEofOutputInvalidated {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{REOPEN_SEAL_TRANSACTION_INVALIDATED_DETAIL}: {}",
            self.source
        )?;
        if let Some(cleanup) = self.cleanup.as_ref() {
            write!(
                formatter,
                "; {REOPEN_SEAL_CLEANUP_FAILURE_DETAIL}: {cleanup}"
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for KernelEofOutputInvalidated {}

impl FrozenProcessTree {
    pub(crate) fn prepare_kernel_eof_reopen_seal(&mut self) -> Result<KernelEofReopenSealCommit> {
        let (fresh_eof_required, identity) = self.run_kernel_eof_seal_transaction(|tree| {
            let authority = tree.acquire_reopen_seal()?;
            let fresh_eof_required = tree.reopen_seal_commit_requires_fresh_eof();
            let identity = authority.committed_identity();
            tree.install_reopen_seal(authority)?;
            tree.commit_reopen_seal_for_destruction()?;
            tree.revalidate_committed_reopen_seal(&identity)?;
            Ok((fresh_eof_required, identity))
        })?;
        Ok(if fresh_eof_required {
            KernelEofReopenSealCommit::FreshEofRequired
        } else {
            KernelEofReopenSealCommit::Reusable(identity)
        })
    }

    fn run_kernel_eof_seal_transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut CapturedProcessTree) -> Result<T>,
    ) -> Result<T> {
        match catch_unwind(AssertUnwindSafe(|| operation(&mut self.tree))) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(source)) => Err(kernel_eof_output_invalidated(
                source,
                self.tree
                    .resolve_interrupted_reopen_seal(ReopenSealRecoveryIntent::RollbackBeforeFence)
                    .err(),
            )),
            Err(payload) => {
                self.tree.trace_interrupted_reopen_seal_cleanup(
                    ReopenSealRecoveryIntent::RollbackBeforeFence,
                );
                std::panic::resume_unwind(payload)
            }
        }
    }

    pub(in crate::process::captured_tree) fn revalidate_kernel_eof_reopen_seal(
        &mut self,
        authority: &CommittedPtyReopenSealAuthority,
    ) -> Result<()> {
        let Err(source) = self.tree.revalidate_committed_reopen_seal(authority) else {
            return Ok(());
        };
        Err(kernel_eof_output_invalidated(
            source,
            self.tree
                .resolve_interrupted_reopen_seal(ReopenSealRecoveryIntent::CommitAfterFence)
                .err(),
        ))
    }

    pub fn kernel_eof_error_invalidates_output(error: &anyhow::Error) -> bool {
        error.downcast_ref::<KernelEofOutputInvalidated>().is_some()
    }

    pub(in crate::process::captured_tree) fn kernel_eof_output_invalidating_error(
        source: anyhow::Error,
    ) -> anyhow::Error {
        kernel_eof_output_invalidated(source, None)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn kernel_eof_invalidating_error_for_test() -> anyhow::Error {
        kernel_eof_output_invalidated(anyhow::anyhow!(TEST_REOPEN_SEAL_INVALIDATED_DETAIL), None)
    }

    pub fn kernel_eof_reopen_seal(ready: &KernelEofKillReady) -> &CommittedPtyReopenSealAuthority {
        &ready.seal
    }
}

fn kernel_eof_output_invalidated(
    source: anyhow::Error,
    cleanup: Option<anyhow::Error>,
) -> anyhow::Error {
    KernelEofOutputInvalidated {
        source: consume_error_detail(source, REOPEN_SEAL_SOURCE_FORMATTING_PANIC_DETAIL),
        cleanup: cleanup.map(consume_failure_detail),
    }
    .into()
}

impl CapturedProcessTree {
    fn reopen_seal_commit_requires_fresh_eof(&self) -> bool {
        self.pty_proof.as_ref().is_some_and(
            super::super::super::PtySessionProof::reopen_seal_commit_requires_fresh_eof,
        )
    }

    pub(in crate::process::captured_tree) fn acquire_reopen_seal(
        &mut self,
    ) -> Result<PtyReopenSealAuthority> {
        self.pty_proof.as_mut().map_or_else(
            || Ok(PtyReopenSealAuthority::missing_proof()),
            super::super::super::PtySessionProof::acquire_reopen_seal,
        )
    }

    pub(in crate::process::captured_tree) fn install_reopen_seal(
        &mut self,
        authority: PtyReopenSealAuthority,
    ) -> Result<()> {
        match self.pty_proof.as_mut() {
            Some(proof) => proof.install_reopen_seal(authority),
            None => authority.validate_missing_proof(),
        }
    }

    pub(in crate::process::captured_tree) fn mark_reopen_seal_destructive_fence_crossed(
        &mut self,
    ) -> Result<()> {
        if let Some(proof) = self.pty_proof.as_mut() {
            proof.mark_reopen_seal_destructive_fence_crossed()?;
        }
        Ok(())
    }

    pub(in crate::process::captured_tree) fn commit_reopen_seal_for_destruction(
        &mut self,
    ) -> Result<()> {
        if let Some(proof) = self.pty_proof.as_mut() {
            proof.commit_reopen_seal_for_destruction()?;
        }
        Ok(())
    }

    pub(in crate::process::captured_tree) fn revalidate_committed_reopen_seal(
        &self,
        authority: &CommittedPtyReopenSealAuthority,
    ) -> Result<()> {
        match self.pty_proof.as_ref() {
            Some(proof) => proof.revalidate_committed_reopen_seal(authority),
            None => authority.validate_missing_proof(),
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/force/captured_tree_force_kernel_eof_seal_terminal_tests.rs"]
mod tests;
