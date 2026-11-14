use super::super::broker_paths::BrokerPaths;
use super::super::constants::*;
use super::super::namespace::BrokerNamespace;
use crate::coordination::CoordinationLock;
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::future::Future;

pub struct BrokerReconcileGuard {
    _lock: CoordinationLock,
    namespace: BrokerNamespace,
}

impl BrokerReconcileGuard {
    pub async fn acquire(paths: &BrokerPaths, deadline: tokio::time::Instant) -> Result<Self> {
        Self::acquire_after_lock(paths, deadline, || async { Ok(()) }).await
    }

    async fn acquire_after_lock<F, Fut>(
        paths: &BrokerPaths,
        deadline: tokio::time::Instant,
        after_lock: F,
    ) -> Result<Self>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let mut after_lock = Some(after_lock);
        loop {
            super::super::deadline_fence::require_before(deadline, BROKER_RECONCILE_LOCK_TIMEOUT)?;
            if let Some(lock) = CoordinationLock::try_acquire(
                paths.reconcile_lock(),
                BROKER_COORDINATION_LOCK_MAX_BYTES,
            )? {
                after_lock.take().context(BROKER_RECONCILIATION_LOCK_BUSY)?().await?;
                super::super::deadline_fence::require_before(
                    deadline,
                    BROKER_RECONCILE_LOCK_TIMEOUT,
                )?;
                let namespace = BrokerNamespace::capture(paths)?;
                super::super::deadline_fence::require_before(
                    deadline,
                    BROKER_RECONCILE_LOCK_TIMEOUT,
                )?;
                return Ok(Self {
                    _lock: lock,
                    namespace,
                });
            }
            super::super::deadline_fence::wait_for_next_attempt(
                deadline,
                BROKER_READY_POLL_INTERVAL,
                BROKER_RECONCILE_LOCK_TIMEOUT,
            )
            .await?;
        }
    }

    #[cfg(test)]
    pub(in crate::remote_broker) async fn acquire_with_deadline_hook<F, Fut>(
        paths: &BrokerPaths,
        deadline: tokio::time::Instant,
        after_lock: F,
    ) -> Result<Self>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        Self::acquire_after_lock(paths, deadline, after_lock).await
    }

    pub fn verify(&self) -> Result<()> {
        self.namespace.verify()
    }

    pub(in crate::remote_broker) fn state_directory(&self) -> Result<PrivateDirectory> {
        self.verify()?;
        self.namespace.state_directory()
    }
}

#[cfg(test)]
pub(super) async fn acquire_reconcile_with_hook<F>(
    paths: &BrokerPaths,
    hook: F,
) -> Result<BrokerReconcileGuard>
where
    F: FnOnce() -> Result<()>,
{
    let lock = super::super::coordination_hook::try_acquire_with_hook(
        paths.reconcile_lock(),
        BROKER_COORDINATION_LOCK_MAX_BYTES,
        hook,
    )?
    .context(BROKER_RECONCILIATION_LOCK_BUSY)?;
    let namespace = BrokerNamespace::capture(paths)?;
    Ok(BrokerReconcileGuard {
        _lock: lock,
        namespace,
    })
}
