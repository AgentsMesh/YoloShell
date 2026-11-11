use crate::pending::{PendingObservation, PendingSpawnHandle};
use anyhow::{Context, Result};
use std::time::Duration;
use yolo_broker_child_custody::combine_cleanup_failure;

const EARLY_EXIT_PREFIX: &str = "broker child exited before readiness: ";
const ATTEMPT_INCREMENT: usize = 1;
const ATTEMPT_OVERFLOW_DETAIL: &str = "broker spawn attempt counter overflow";
const RETRY_DEADLINE_OVERFLOW_DETAIL: &str = "broker spawn retry deadline overflow";
const PENDING_COMPLETION_DETAIL: &str = "broker launch remained pending after reconciliation";
const PENDING_CUSTODY_MISSING_DETAIL: &str = "pending broker launch custody is missing";

pub(super) struct SpawnAuthority {
    pending: Option<PendingSpawnHandle>,
    next_admission: tokio::time::Instant,
    retry_interval: Duration,
    attempts: usize,
    last_exit_error: Option<String>,
    last_spawn_error: Option<String>,
}

impl SpawnAuthority {
    pub(super) fn new(retry_interval: Duration) -> Self {
        Self {
            pending: None,
            next_admission: tokio::time::Instant::now(),
            retry_interval,
            attempts: usize::default(),
            last_exit_error: None,
            last_spawn_error: None,
        }
    }

    pub(super) fn refresh(&mut self) -> Result<()> {
        let Some(pending) = self.pending.as_mut() else {
            return Ok(());
        };
        let observation = pending.observe();
        match observation {
            Ok(PendingObservation::Pending) => {}
            Ok(PendingObservation::Exited(detail)) => {
                let early_exit = format!("{EARLY_EXIT_PREFIX}{detail}");
                let cleanup = self.take_pending()?.compensate();
                if cleanup.is_err() {
                    return Err(combine_cleanup_failure(
                        anyhow::anyhow!(early_exit),
                        cleanup,
                    ));
                }
                self.last_exit_error = Some(early_exit);
            }
            Err(observation) => {
                let cleanup = self.take_pending()?.compensate();
                return Err(combine_cleanup_failure(observation, cleanup));
            }
        }
        Ok(())
    }

    pub(super) fn can_admit(&self, now: tokio::time::Instant) -> bool {
        self.pending.is_none() && now >= self.next_admission
    }

    pub(super) fn begin(&mut self) -> Result<()> {
        self.attempts = self
            .attempts
            .checked_add(ATTEMPT_INCREMENT)
            .context(ATTEMPT_OVERFLOW_DETAIL)?;
        Ok(())
    }

    pub(super) fn finish(&mut self, result: Result<PendingSpawnHandle>) -> Result<()> {
        match result {
            Ok(pending) => {
                self.pending = Some(pending);
                self.last_spawn_error = None;
            }
            Err(error) => self.last_spawn_error = Some(error.to_string()),
        }
        self.next_admission = tokio::time::Instant::now()
            .checked_add(self.retry_interval)
            .context(RETRY_DEADLINE_OVERFLOW_DETAIL)?;
        Ok(())
    }

    pub(super) fn commit(&mut self, status: &yolo_remote_broker::BrokerStatus) -> Result<()> {
        match self.pending.take() {
            Some(pending) if pending.matches_status(status) => pending.adopt(),
            Some(pending) => pending.compensate(),
            None => Ok(()),
        }
    }

    pub(super) fn adopt(&mut self, status: &yolo_remote_broker::BrokerStatus) -> Result<bool> {
        let matches = self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.matches_status(status));
        if !matches {
            return Ok(false);
        }
        self.take_pending()?.adopt()?;
        Ok(true)
    }

    pub(super) fn owns(&self, status: &yolo_remote_broker::BrokerStatus) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.matches_status(status))
    }

    pub(super) fn attempts(&self) -> usize {
        self.attempts
    }

    pub(super) fn last_exit_error(&self) -> Option<&str> {
        self.last_exit_error.as_deref()
    }

    pub(super) fn last_spawn_error(&self) -> Option<&str> {
        self.last_spawn_error.as_deref()
    }

    pub(super) fn resolve<T>(mut self, outcome: Result<T>) -> Result<T> {
        let Some(pending) = self.pending.take() else {
            return outcome;
        };
        let cleanup = pending.compensate();
        let primary = match outcome {
            Ok(_) => anyhow::anyhow!(PENDING_COMPLETION_DETAIL),
            Err(error) => error,
        };
        Err(combine_cleanup_failure(primary, cleanup))
    }

    fn take_pending(&mut self) -> Result<PendingSpawnHandle> {
        self.pending.take().context(PENDING_CUSTODY_MISSING_DETAIL)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/start_attempt_tests.rs"]
mod tests;
