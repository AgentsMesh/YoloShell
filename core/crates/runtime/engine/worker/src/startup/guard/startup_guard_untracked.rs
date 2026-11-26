use super::super::constants::{STARTUP_CHILD_EXIT_POLL_INTERVAL, STARTUP_CHILD_EXIT_TIMEOUT};
use super::super::pty_missing_eof::MissingEofDeadline;
use super::super::pty_output_progress::PtyOutputRecovery;
use super::super::startup_untracked_proof::UntrackedStartupProducersGone;
use super::{STARTUP_CHILD_UNAVAILABLE_CONTEXT, StartupChildGuard, StartupOutputProgress};
use crate::process::{DirectChildIdentity, is_wait_authority_loss};
use anyhow::{Context, Result, anyhow};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

pub(crate) const UNTRACKED_POLL_PANIC_DETAIL: &str = "untracked startup child poll panicked";
pub(crate) const UNTRACKED_GATE_PROOF_UNAVAILABLE_CONTEXT: &str =
    "untracked startup child lacks an unreleased bootstrap gate proof";
pub(crate) const UNTRACKED_REAPER_UNAVAILABLE_CONTEXT: &str =
    "untracked startup child reaper reservation is unavailable";
pub(crate) const UNTRACKED_CLEANUP_DEFERRED_DETAIL: &str =
    "untracked startup child cleanup exceeded its inline deadline and was deferred";

impl StartupChildGuard {
    pub(crate) fn terminate_untracked_with_output(
        &mut self,
        failure: Option<anyhow::Error>,
        output: StartupOutputProgress,
    ) -> Result<()> {
        let deadline = Instant::now() + STARTUP_CHILD_EXIT_TIMEOUT;
        self.terminate_untracked_until(failure, output, deadline)
    }

    #[cfg(test)]
    pub(crate) fn terminate_untracked_at_deadline_for_test(&mut self) -> Result<()> {
        let output = self
            .output_progress
            .take()
            .context(super::STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT)?;
        let failure = self.stop_monitor().err();
        self.terminate_untracked_until(failure, output, Instant::now())
    }

    pub(crate) fn terminate_untracked_until(
        &mut self,
        mut failure: Option<anyhow::Error>,
        mut output: StartupOutputProgress,
        deadline: Instant,
    ) -> Result<()> {
        match self.promote_untracked_child() {
            Ok(promoted) => return promoted.settle(self, output, failure),
            Err(error) => super::cleanup::remember(&mut failure, error),
        }
        let mut reader_recovery = match &output {
            StartupOutputProgress::Exclusive(_) => None,
            StartupOutputProgress::ReaderBacked(output) => Some(output.recovery()),
        };
        let mut wait_authority_lost = false;
        let mut missing_eof = MissingEofDeadline::production();
        loop {
            let settled = self.advance_untracked(
                &mut failure,
                &mut wait_authority_lost,
                &mut output,
                &mut reader_recovery,
                &mut missing_eof,
            );
            if settled {
                self.clear();
                return failure.map_or(Ok(()), Err);
            }
            if Instant::now() >= deadline {
                drop(reader_recovery);
                return self.defer_untracked(failure, output, wait_authority_lost, missing_eof);
            }
            std::thread::sleep(STARTUP_CHILD_EXIT_POLL_INTERVAL);
        }
    }

    pub(crate) fn advance_untracked(
        &mut self,
        failure: &mut Option<anyhow::Error>,
        wait_authority_lost: &mut bool,
        output: &mut StartupOutputProgress,
        reader: &mut Option<PtyOutputRecovery>,
        missing_eof: &mut MissingEofDeadline,
    ) -> bool {
        let output_complete = super::untracked_progress::contain_progress(failure, || {
            super::untracked_progress::progress_output(output, reader)
        });
        if self.untracked_gone_committed {
            return output_complete
                || super::untracked_progress::contain_missing_eof(
                    failure,
                    missing_eof,
                    output,
                    reader,
                );
        }
        if self.live_identity.is_none() && !*wait_authority_lost {
            *wait_authority_lost = self.try_bind_untracked_identity(failure);
        }
        let mut identity_gone = false;
        if !*wait_authority_lost {
            match self.poll_untracked() {
                Ok(true) => identity_gone = true,
                Ok(false) => {}
                Err(error) => {
                    *wait_authority_lost = is_wait_authority_loss(&error);
                    super::cleanup::remember(failure, error);
                }
            }
        }
        if *wait_authority_lost {
            identity_gone = self.untracked_identity_is_gone(failure);
        }
        if !identity_gone {
            return false;
        }
        let Some(gate) = self.unreleased_gate.take() else {
            super::cleanup::remember(failure, anyhow!(UNTRACKED_GATE_PROOF_UNAVAILABLE_CONTEXT));
            return false;
        };
        self.untracked_gone_committed = true;
        let proof = UntrackedStartupProducersGone::from_exited_closed_gate(gate);
        super::untracked_progress::contain_progress(failure, || {
            super::untracked_progress::complete_output(output, reader, proof)
        })
    }

    #[cfg(test)]
    pub(crate) fn advance_untracked_for_test(
        &mut self,
        failure: &mut Option<anyhow::Error>,
        wait_authority_lost: &mut bool,
        output: &mut StartupOutputProgress,
        reader: &mut Option<PtyOutputRecovery>,
        missing_eof: &mut MissingEofDeadline,
    ) -> bool {
        self.advance_untracked(failure, wait_authority_lost, output, reader, missing_eof)
    }

    pub(crate) fn try_bind_untracked_identity(
        &mut self,
        failure: &mut Option<anyhow::Error>,
    ) -> bool {
        let result = (|| {
            let Some(_spawn_guard) = crate::process::try_waitable_child_spawn_guard()? else {
                return Ok(None);
            };
            let child = self
                .child
                .as_ref()
                .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
            let identity = DirectChildIdentity::capture_candidate(child)?;
            identity.prove_current(child)?;
            Ok::<_, anyhow::Error>(Some(identity))
        })();
        match result {
            Ok(Some(identity)) => {
                self.process_id = Some(identity.root().pid);
                self.identity = Some(identity.root());
                self.live_identity = Some(identity.live_root());
                false
            }
            Ok(None) => false,
            Err(error) => {
                let authority_lost = is_wait_authority_loss(&error);
                super::cleanup::remember(failure, error);
                authority_lost
            }
        }
    }

    pub(crate) fn untracked_identity_is_gone(&self, failure: &mut Option<anyhow::Error>) -> bool {
        let Some(identity) = self.live_identity.as_ref() else {
            return false;
        };
        match identity.try_matches() {
            Ok(matches) => !matches,
            Err(error) => {
                super::cleanup::remember(failure, error);
                false
            }
        }
    }

    pub(crate) fn poll_untracked(&mut self) -> Result<bool> {
        let child = self
            .child
            .as_mut()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        catch_unwind(AssertUnwindSafe(|| child.try_wait()))
            .map_err(|payload| {
                let detail = super::super::supervision::panic_detail(payload);
                anyhow!("{UNTRACKED_POLL_PANIC_DETAIL}: {detail}")
            })?
            .map(|status| status.is_some())
            .map_err(Into::into)
    }
}
