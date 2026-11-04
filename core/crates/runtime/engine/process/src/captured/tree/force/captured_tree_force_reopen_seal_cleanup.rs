use crate::process::captured_tree::CapturedProcessTree;
use anyhow::{Context, Result};
use std::panic::{AssertUnwindSafe, catch_unwind};

const REOPEN_SEAL_RECOVERY_ATTEMPTS: usize = 3;
const REOPEN_SEAL_CLEANUP_FAILURE_DETAIL: &str =
    "failed to resolve an interrupted PTY reopen-seal transaction";
const REOPEN_SEAL_CLEANUP_PANIC_DETAIL: &str =
    "PTY reopen-seal cleanup panicked after transaction failure";
const REOPEN_SEAL_CLEANUP_EXHAUSTED_DETAIL: &str =
    "PTY reopen-seal recovery exhausted its retry budget and was quarantined";
const REOPEN_SEAL_QUARANTINE_FAILURE_DETAIL: &str =
    "failed to establish a physical PTY reopen-seal quarantine";
#[derive(Clone, Copy)]
pub(super) enum ReopenSealRecoveryIntent {
    RollbackBeforeFence,
    CommitAfterFence,
}

impl CapturedProcessTree {
    pub(super) fn resolve_interrupted_reopen_seal(
        &mut self,
        intent: ReopenSealRecoveryIntent,
    ) -> Result<()> {
        let Some(proof) = self.pty_proof.as_mut() else {
            return Ok(());
        };
        let mut last_failure = None;
        for _attempt in 0..REOPEN_SEAL_RECOVERY_ATTEMPTS {
            if matches!(intent, ReopenSealRecoveryIntent::RollbackBeforeFence) {
                match catch_cleanup(|| proof.rollback_reopen_seal_before_commit()) {
                    Ok(()) => return recovery_succeeded(&mut last_failure),
                    Err(error) => {
                        super::reopen_seal_terminal::remember_failure(&mut last_failure, error)
                    }
                }
            }
            match catch_cleanup(|| proof.settle_reopen_seal_after_fence()) {
                Ok(()) => return recovery_succeeded(&mut last_failure),
                Err(error) => {
                    super::reopen_seal_terminal::remember_failure(&mut last_failure, error)
                }
            }
        }
        if let Err(error) = catch_cleanup(|| proof.quarantine_reopen_seal()) {
            super::reopen_seal_terminal::remember_failure(&mut last_failure, error);
            let failure = last_failure.expect(REOPEN_SEAL_CLEANUP_EXHAUSTED_DETAIL);
            return Err(failure).context(REOPEN_SEAL_QUARANTINE_FAILURE_DETAIL);
        }
        let failure = last_failure.expect(REOPEN_SEAL_CLEANUP_EXHAUSTED_DETAIL);
        Err(failure).context(REOPEN_SEAL_CLEANUP_EXHAUSTED_DETAIL)
    }

    pub(super) fn trace_interrupted_reopen_seal_cleanup(
        &mut self,
        intent: ReopenSealRecoveryIntent,
    ) {
        if let Err(error) = self.resolve_interrupted_reopen_seal(intent) {
            super::reopen_seal_terminal::report_failure(error, REOPEN_SEAL_CLEANUP_FAILURE_DETAIL);
        }
    }
}

fn recovery_succeeded(last_failure: &mut Option<anyhow::Error>) -> Result<()> {
    super::reopen_seal_terminal::discard_failure(last_failure);
    Ok(())
}

fn catch_cleanup(operation: impl FnOnce() -> Result<()>) -> Result<()> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(payload) => {
            super::reopen_seal_terminal::dispose_payload(payload);
            anyhow::bail!(REOPEN_SEAL_CLEANUP_PANIC_DETAIL)
        }
    }
}
