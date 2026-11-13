use super::{AuthorizedBrokerStartup, BrokerLaunchAuthority};
use crate::remote_broker::instance_lock::BrokerInstanceLock;
use crate::remote_broker::{BROKER_READY_POLL_INTERVAL, BROKER_READY_TIMEOUT, BrokerPaths};
use anyhow::{Context, Result};
use std::future::{Future, ready};

const STARTUP_DEADLINE_OVERFLOW: &str = "broker startup lock deadline overflow";
pub(super) const STARTUP_LOCK_TIMEOUT: &str = "timed out waiting for broker startup lock";

impl AuthorizedBrokerStartup {
    pub fn acquire(paths: BrokerPaths, authority: BrokerLaunchAuthority) -> Result<Self> {
        let deadline = startup_deadline()?;
        let lock = BrokerInstanceLock::acquire(paths.instance_lock())?;
        authority.verify(&paths)?;
        crate::remote_broker::deadline_fence::require_before(deadline, STARTUP_LOCK_TIMEOUT)?;
        Ok(Self {
            paths,
            authority,
            lock,
            deadline,
        })
    }

    pub async fn acquire_for_launch(
        paths: BrokerPaths,
        authority: BrokerLaunchAuthority,
        deadline: tokio::time::Instant,
    ) -> Result<Self> {
        Self::acquire_for_launch_after_lock(paths, authority, deadline, || ready(Ok(()))).await
    }

    async fn acquire_for_launch_after_lock<F, Fut>(
        paths: BrokerPaths,
        authority: BrokerLaunchAuthority,
        deadline: tokio::time::Instant,
        after_lock: F,
    ) -> Result<Self>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let mut after_lock = Some(after_lock);
        loop {
            crate::remote_broker::deadline_fence::require_before(deadline, STARTUP_LOCK_TIMEOUT)?;
            authority.verify(&paths)?;
            if let Some(lock) = BrokerInstanceLock::try_acquire(paths.instance_lock())? {
                after_lock.take().context(STARTUP_LOCK_TIMEOUT)?().await?;
                authority.verify(&paths)?;
                crate::remote_broker::deadline_fence::require_before(
                    deadline,
                    STARTUP_LOCK_TIMEOUT,
                )?;
                return Ok(Self {
                    paths,
                    authority,
                    lock,
                    deadline,
                });
            }
            crate::remote_broker::deadline_fence::wait_for_next_attempt(
                deadline,
                BROKER_READY_POLL_INTERVAL,
                STARTUP_LOCK_TIMEOUT,
            )
            .await?;
        }
    }

    #[cfg(test)]
    pub(in crate::remote_broker) async fn acquire_for_launch_with_deadline_hook<F, Fut>(
        paths: BrokerPaths,
        authority: BrokerLaunchAuthority,
        deadline: tokio::time::Instant,
        after_lock: F,
    ) -> Result<Self>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        Self::acquire_for_launch_after_lock(paths, authority, deadline, after_lock).await
    }

    pub fn verify(&self) -> Result<()> {
        crate::remote_broker::deadline_fence::require_before(self.deadline, STARTUP_LOCK_TIMEOUT)?;
        self.authority.verify(&self.paths)?;
        crate::remote_broker::deadline_fence::require_before(self.deadline, STARTUP_LOCK_TIMEOUT)
    }

    pub(in crate::remote_broker) fn into_parts(
        self,
    ) -> (
        BrokerPaths,
        BrokerLaunchAuthority,
        BrokerInstanceLock,
        tokio::time::Instant,
    ) {
        (self.paths, self.authority, self.lock, self.deadline)
    }
}

fn startup_deadline() -> Result<tokio::time::Instant> {
    tokio::time::Instant::now()
        .checked_add(BROKER_READY_TIMEOUT)
        .context(STARTUP_DEADLINE_OVERFLOW)
}
