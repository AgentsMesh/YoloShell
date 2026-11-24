use super::super::constants::STARTUP_CHILD_EXIT_POLL_INTERVAL;
use super::super::pty_missing_eof::poll_output;
use super::super::startup_untracked_proof::UntrackedStartupProducersGone;
use super::super::supervision;
use super::StartupUntrackedCleanup;
use super::driver_outcome;
use crate::process::{DirectChildIdentity, ProcessIdentityObservation, is_wait_authority_loss};
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

pub(crate) const RETRY_MAX_INTERVAL: Duration = Duration::from_secs(1);
pub(crate) const RETRY_MULTIPLIER: u32 = 2;
pub(crate) const BIND_PANIC_DETAIL: &str = "untracked startup reaper identity binding panicked";
pub(crate) const PROGRESS_PANIC_DETAIL: &str = "untracked startup reaper output progress panicked";
pub(crate) const POLL_PANIC_DETAIL: &str = "untracked startup reaper child poll panicked";
pub(crate) const CLOSED_GATE_PROOF_MISSING_DETAIL: &str =
    "untracked startup reaper lacks an unreleased bootstrap gate proof";

impl StartupUntrackedCleanup {
    pub(crate) fn reap(&mut self) {
        #[cfg(feature = "startup-handoff-test-hooks")]
        super::super::startup_handoff_test_hook::publish_handoff();
        let mut delay = STARTUP_CHILD_EXIT_POLL_INTERVAL;
        loop {
            if self.drive_once() {
                #[cfg(feature = "startup-handoff-test-hooks")]
                super::super::startup_handoff_test_hook::complete_settlement();
                return;
            }
            std::thread::sleep(delay);
            delay = delay
                .saturating_mul(RETRY_MULTIPLIER)
                .min(RETRY_MAX_INTERVAL);
        }
    }

    pub(crate) fn drive_once(&mut self) -> bool {
        let output_complete = self.progress();
        if self.producers_gone_committed {
            return output_complete || self.settle_missing_eof();
        }
        self.bind_identity();
        let mut identity_gone = false;
        if !self.wait_authority_lost {
            identity_gone = self.poll_child();
        }
        if self.wait_authority_lost {
            identity_gone = self.identity_is_gone();
        }
        if !identity_gone {
            return false;
        }
        let Some(gate) = self.unreleased_gate.take() else {
            super::super::terminal_reporting::warn_message(CLOSED_GATE_PROOF_MISSING_DETAIL);
            return false;
        };
        self.producers_gone_committed = true;
        let proof = UntrackedStartupProducersGone::from_exited_closed_gate(gate);
        self.complete(proof)
    }

    #[cfg(test)]
    pub(crate) fn drive_once_for_test(&mut self) -> bool {
        self.drive_once()
    }

    pub(crate) fn bind_identity(&mut self) {
        if self.identity.live().is_some() || self.wait_authority_lost {
            return;
        }
        let attempted = contain(BIND_PANIC_DETAIL, || {
            let Some(_spawn_guard) = crate::process::try_waitable_child_spawn_guard()? else {
                return Ok(None);
            };
            let identity = DirectChildIdentity::capture_candidate(&self.child)?;
            identity.prove_current(&self.child)?;
            Ok(Some(identity))
        });
        match attempted {
            Some(Ok(Some(identity))) => self.identity.bind(identity),
            Some(Ok(None)) => {}
            Some(Err(error)) => {
                self.wait_authority_lost =
                    supervision::contain(|| is_wait_authority_loss(&error)).unwrap_or(false);
                driver_outcome::warn(error);
            }
            None => {}
        }
    }

    pub(crate) fn identity_is_gone(&self) -> bool {
        if let Some(identity) = self.identity.live() {
            return driver_outcome::identity_gone(contain(BIND_PANIC_DETAIL, || {
                identity.try_matches()
            }));
        }
        let Some(identity) = self.identity.durable() else {
            return false;
        };
        matches!(
            contain(BIND_PANIC_DETAIL, || Ok(identity.observe())),
            Some(Ok(ProcessIdentityObservation::Gone))
        )
    }

    pub(crate) fn progress(&mut self) -> bool {
        driver_outcome::output_complete(contain(PROGRESS_PANIC_DETAIL, || self.progress_output()))
    }

    pub(crate) fn complete(&mut self, proof: UntrackedStartupProducersGone) -> bool {
        driver_outcome::output_complete(contain(PROGRESS_PANIC_DETAIL, || {
            self.complete_output(proof)
        }))
    }

    pub(crate) fn settle_missing_eof(&mut self) -> bool {
        let result = match &mut self.output {
            super::super::startup_guard::StartupOutputProgress::Exclusive(output) => {
                contain(PROGRESS_PANIC_DETAIL, || {
                    poll_output(&mut self.missing_eof, true, output, Instant::now())
                })
            }
            super::super::startup_guard::StartupOutputProgress::ReaderBacked(_) => {
                contain(PROGRESS_PANIC_DETAIL, || {
                    poll_output(
                        &mut self.missing_eof,
                        true,
                        self.reader_recovery
                            .as_mut()
                            .expect(super::READER_RECOVERY_UNAVAILABLE_CONTEXT),
                        Instant::now(),
                    )
                })
            }
        };
        driver_outcome::missing_eof_settled(result)
    }

    pub(crate) fn poll_child(&mut self) -> bool {
        let result = contain(POLL_PANIC_DETAIL, || {
            self.child.try_wait().map_err(Into::into)
        });
        driver_outcome::child_exited(result, &mut self.wait_authority_lost)
    }
}

pub(crate) fn contain<Value>(
    panic_context: &'static str,
    callback: impl FnOnce() -> Result<Value>,
) -> Option<Result<Value>> {
    match catch_unwind(AssertUnwindSafe(callback)) {
        Ok(result) => Some(result),
        Err(payload) => {
            let panic_detail = supervision::panic_detail(payload);
            let detail = format!("{panic_context}: {panic_detail}");
            super::super::terminal_reporting::warn_detail(
                &detail,
                driver_outcome::CALLBACK_ERROR_DETAIL,
            );
            None
        }
    }
}
