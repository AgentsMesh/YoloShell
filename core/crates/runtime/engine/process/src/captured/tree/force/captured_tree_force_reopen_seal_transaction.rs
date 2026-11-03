use super::commit::closure_remains_frozen;
use super::reopen_seal_cleanup::ReopenSealRecoveryIntent;
use super::reopen_seal_terminal::consume_failure_detail;
use super::{FrozenKillBasis, FrozenProcessTree, KillCommitReady, OrdinaryKillReady};
use crate::process::captured_tree::termination;
use anyhow::{Error, Result};
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

const REOPEN_SEAL_TRANSACTION_CLEANUP_CONTEXT: &str =
    "failed to settle interrupted kill reopen-seal transaction";

enum KillReopenSealOutcome {
    Settled(KillCommitReady),
    Abort(KillCommitReady),
}

impl FrozenProcessTree {
    pub(super) fn prepare_exact_kill_transaction(
        &mut self,
        commit_fence: &mut impl FnMut() -> Result<()>,
        on_destructive_commit: &mut impl FnMut(),
    ) -> Result<KillCommitReady> {
        self.run_kill_reopen_seal_transaction(|tree, intent| {
            let reopen_seal = tree.tree.acquire_reopen_seal()?;
            if !closure_remains_frozen(tree)? {
                return Ok(KillReopenSealOutcome::Abort(KillCommitReady::Refreeze));
            }
            tree.tree.install_reopen_seal(reopen_seal)?;
            tree.cross_destructive_commit_boundary(intent, on_destructive_commit)?;
            commit_fence()?;
            tree.tree.commit_reopen_seal_for_destruction()?;
            let Some(delivery) = termination::signal_exact(
                &tree.tree,
                &tree.closure,
                tree.version.clone(),
                match tree.kill_basis {
                    FrozenKillBasis::ExactPositive => {
                        termination::ExactKillAuthority::ExactPositive
                    }
                    FrozenKillBasis::IncrementalExactPositive => {
                        termination::ExactKillAuthority::IncrementalExactPositive
                    }
                    FrozenKillBasis::Ordinary | FrozenKillBasis::KernelEof => unreachable!(),
                },
            )?
            else {
                return Ok(KillReopenSealOutcome::Settled(KillCommitReady::Refreeze));
            };
            let ready = match tree.kill_basis {
                FrozenKillBasis::ExactPositive => KillCommitReady::PendingExact(delivery),
                FrozenKillBasis::IncrementalExactPositive => {
                    KillCommitReady::PendingIncremental(delivery)
                }
                FrozenKillBasis::Ordinary | FrozenKillBasis::KernelEof => unreachable!(),
            };
            Ok(KillReopenSealOutcome::Settled(ready))
        })
    }

    pub(super) fn prepare_ordinary_kill_transaction(
        &mut self,
        commit_fence: &mut impl FnMut() -> Result<()>,
        on_destructive_commit: &mut impl FnMut(),
    ) -> Result<KillCommitReady> {
        if self.closure.is_empty() {
            return Ok(if closure_remains_frozen(self)? {
                KillCommitReady::PendingOrdinary(OrdinaryKillReady {
                    version: self.version.clone(),
                })
            } else {
                KillCommitReady::Refreeze
            });
        }
        self.run_kill_reopen_seal_transaction(|tree, intent| {
            let reopen_seal = tree.tree.acquire_reopen_seal()?;
            if !closure_remains_frozen(tree)? {
                return Ok(KillReopenSealOutcome::Abort(KillCommitReady::Refreeze));
            }
            tree.tree.install_reopen_seal(reopen_seal)?;
            tree.cross_destructive_commit_boundary(intent, on_destructive_commit)?;
            commit_fence()?;
            tree.tree.commit_reopen_seal_for_destruction()?;
            termination::signal(&tree.tree)?;
            Ok(KillReopenSealOutcome::Settled(
                KillCommitReady::PendingOrdinary(OrdinaryKillReady {
                    version: tree.version.clone(),
                }),
            ))
        })
    }

    fn cross_destructive_commit_boundary(
        &mut self,
        intent: &Cell<ReopenSealRecoveryIntent>,
        on_destructive_commit: &mut impl FnMut(),
    ) -> Result<()> {
        self.tree.mark_reopen_seal_destructive_fence_crossed()?;
        intent.set(ReopenSealRecoveryIntent::CommitAfterFence);
        self.destructive_started = true;
        on_destructive_commit();
        Ok(())
    }

    fn run_kill_reopen_seal_transaction(
        &mut self,
        operation: impl FnOnce(
            &mut Self,
            &Cell<ReopenSealRecoveryIntent>,
        ) -> Result<KillReopenSealOutcome>,
    ) -> Result<KillCommitReady> {
        let intent = Cell::new(ReopenSealRecoveryIntent::RollbackBeforeFence);
        let outcome = catch_unwind(AssertUnwindSafe(|| operation(self, &intent)));
        match outcome {
            Ok(Ok(KillReopenSealOutcome::Settled(ready))) => Ok(ready),
            Ok(Ok(KillReopenSealOutcome::Abort(ready))) => {
                self.tree.resolve_interrupted_reopen_seal(intent.get())?;
                Ok(ready)
            }
            Ok(Err(source)) => {
                let cleanup = self.tree.resolve_interrupted_reopen_seal(intent.get());
                Err(failed_transaction_error(source, cleanup))
            }
            Err(payload) => {
                self.tree
                    .trace_interrupted_reopen_seal_cleanup(intent.get());
                std::panic::resume_unwind(payload)
            }
        }
    }
}

fn failed_transaction_error(source: Error, cleanup: Result<()>) -> Error {
    let Err(cleanup) = cleanup else {
        return source;
    };
    let detail = consume_failure_detail(cleanup);
    source.context(format!(
        "{REOPEN_SEAL_TRANSACTION_CLEANUP_CONTEXT}: {detail}"
    ))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/force/captured_tree_force_reopen_seal_transaction_tests.rs"]
mod tests;
