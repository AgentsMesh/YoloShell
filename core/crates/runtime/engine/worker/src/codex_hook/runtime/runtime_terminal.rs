use super::super::authority::CommittedProcessExit;
use super::super::constants::CODEX_STORAGE_LATCHED_ERROR;
use super::super::error::CodexHookRejection;
use super::super::publication::{CodexProcessExitPublication, ReplayReport};
use super::CodexHookRuntime;
use crate::agent::hooks::ProcessIdentity;
use anyhow::{Result, ensure};

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn reconcile_pending(&self) -> Result<ReplayReport> {
        self.contain_authority_unwind(
            || {
                let mut lifecycle = self.lock_lifecycle_anyhow()?;
                #[cfg(test)]
                self.panic_reconcile_for_test();
                ensure!(!lifecycle.storage_failed(), CODEX_STORAGE_LATCHED_ERROR);
                let operation_proof = match lifecycle.prove_snapshot_current() {
                    Ok(proof) => proof,
                    Err(error) => {
                        return Err(self.contain_authority_error(&mut lifecycle, error, None));
                    }
                };
                if let Err(error) = lifecycle.ensure_operable() {
                    return Err(self.contain_authority_error(
                        &mut lifecycle,
                        error,
                        Some(&operation_proof),
                    ));
                }
                drop(operation_proof);
                let result =
                    super::super::outbox_authority::with_locked_state(&self.outbox, |state| {
                        super::super::replay::reconcile(&mut lifecycle, state)
                    });
                let publication_proof = match lifecycle.prove_snapshot_current() {
                    Ok(proof) => proof,
                    Err(error) => {
                        return Err(self.contain_authority_error(&mut lifecycle, error, None));
                    }
                };
                match result {
                    Ok(report) => {
                        if let Err(error) = self.publish_lifecycle(&publication_proof, &lifecycle) {
                            return Err(self.contain_authority_error(&mut lifecycle, error, None));
                        }
                        Ok(report)
                    }
                    Err(error) => Err(self.contain_authority_error(
                        &mut lifecycle,
                        error,
                        Some(&publication_proof),
                    )),
                }
            },
            || anyhow::anyhow!(CODEX_STORAGE_LATCHED_ERROR),
        )
    }

    pub(in crate::worker::codex_hook) fn observe_process_exit(
        &self,
        observed: ProcessIdentity,
    ) -> Result<CodexProcessExitPublication, CodexHookRejection> {
        self.contain_authority_unwind(
            || {
                let mut lifecycle = self.lock_lifecycle()?;
                #[cfg(test)]
                self.panic_process_exit_for_test();
                if lifecycle.storage_failed() {
                    return Err(CodexHookRejection::Storage(anyhow::anyhow!(
                        CODEX_STORAGE_LATCHED_ERROR
                    )));
                }
                let operation_proof = match lifecycle.prove_snapshot_current() {
                    Ok(proof) => proof,
                    Err(error) => {
                        return Err(self.reject_authority_error(&mut lifecycle, error, None));
                    }
                };
                if let Err(error) = lifecycle.ensure_operable() {
                    return Err(self.reject_authority_error(
                        &mut lifecycle,
                        error,
                        Some(&operation_proof),
                    ));
                }
                drop(operation_proof);
                let result =
                    super::super::outbox_authority::with_locked_state(&self.outbox, |state| {
                        let replay = super::super::replay::reconcile(&mut lifecycle, state)?;
                        Ok((replay, lifecycle.process_exit_committed(observed)?))
                    });
                let publication_proof = match lifecycle.prove_snapshot_current() {
                    Ok(proof) => proof,
                    Err(error) => {
                        return Err(self.reject_authority_error(&mut lifecycle, error, None));
                    }
                };
                match result {
                    Ok((replay, CommittedProcessExit::Accepted(disposition, _state))) => {
                        if let Err(error) = self.publish_lifecycle(&publication_proof, &lifecycle) {
                            return Err(self.reject_authority_error(&mut lifecycle, error, None));
                        }
                        Ok(CodexProcessExitPublication {
                            disposition,
                            replay,
                        })
                    }
                    Ok((_, CommittedProcessExit::Rejected(violation, _state))) => {
                        if let Err(error) = self.publish_lifecycle(&publication_proof, &lifecycle) {
                            return Err(self.reject_authority_error(&mut lifecycle, error, None));
                        }
                        Err(CodexHookRejection::Violation(violation))
                    }
                    Err(error) => Err(self.reject_authority_error(
                        &mut lifecycle,
                        error,
                        Some(&publication_proof),
                    )),
                }
            },
            || CodexHookRejection::Storage(anyhow::anyhow!(CODEX_STORAGE_LATCHED_ERROR)),
        )
    }
}
