use super::{BROKER_READY_POLL_INTERVAL, BrokerPaths};
use crate::coordination::CoordinationLock;
use anyhow::{Context, Result};
use std::future::{Future, ready};

const PUBLICATION_GATE_TIMEOUT: &str = "timed out waiting for broker launch publication gate";
const PUBLICATION_GATE_HOOK_MISSING: &str = "broker publication gate hook is missing";

pub(super) struct BrokerLaunchPublicationGuard {
    _lock: CoordinationLock,
}

impl BrokerLaunchPublicationGuard {
    #[cfg(test)]
    pub(super) async fn acquire(
        paths: &BrokerPaths,
        deadline: tokio::time::Instant,
    ) -> Result<Self> {
        Self::acquire_inner(paths, deadline, || Ok(()), || ready(Ok(()))).await
    }

    pub(super) async fn acquire_observed<F>(
        paths: &BrokerPaths,
        deadline: tokio::time::Instant,
        on_contention: F,
    ) -> Result<Self>
    where
        F: FnMut() -> Result<()>,
    {
        Self::acquire_inner(paths, deadline, on_contention, || ready(Ok(()))).await
    }

    async fn acquire_inner<C, A, Fut>(
        paths: &BrokerPaths,
        deadline: tokio::time::Instant,
        mut on_contention: C,
        after_lock: A,
    ) -> Result<Self>
    where
        C: FnMut() -> Result<()>,
        A: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let mut after_lock = Some(after_lock);
        loop {
            super::deadline_fence::require_before(deadline, PUBLICATION_GATE_TIMEOUT)?;
            if let Some(lock) = CoordinationLock::try_acquire(
                paths.launch_publication_lock(),
                super::constants::BROKER_COORDINATION_LOCK_MAX_BYTES,
            )? {
                after_lock.take().context(PUBLICATION_GATE_HOOK_MISSING)?().await?;
                super::deadline_fence::require_before(deadline, PUBLICATION_GATE_TIMEOUT)?;
                return Ok(Self { _lock: lock });
            }
            on_contention()?;
            super::deadline_fence::wait_for_next_attempt(
                deadline,
                BROKER_READY_POLL_INTERVAL,
                PUBLICATION_GATE_TIMEOUT,
            )
            .await?;
        }
    }

    #[cfg(test)]
    pub(super) async fn acquire_with_deadline_hook<A, Fut>(
        paths: &BrokerPaths,
        deadline: tokio::time::Instant,
        after_lock: A,
    ) -> Result<Self>
    where
        A: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        Self::acquire_inner(paths, deadline, || Ok(()), after_lock).await
    }
}
