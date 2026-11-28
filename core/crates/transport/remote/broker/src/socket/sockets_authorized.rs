use super::{BrokerInstanceLock, BrokerLaunchAuthority, BrokerPaths, BrokerSockets};
use crate::remote_broker::publication_gate::BrokerLaunchPublicationGuard;
use anyhow::{Context, Result, ensure};
use std::future::{Future, ready};

const AUTHORIZED_DEADLINE_MISSING: &str = "authorized broker startup deadline is missing";
const AUTHORIZED_DEADLINE_EXPIRED: &str = "authorized broker startup deadline expired";
const UNAUTHORIZED_DEADLINE_PRESENT: &str = "unauthorized broker startup supplied a deadline";

impl BrokerSockets {
    pub(in crate::remote_broker) async fn bind_authorized(
        paths: &BrokerPaths,
        authority: Option<BrokerLaunchAuthority>,
        startup_lock: Option<BrokerInstanceLock>,
        startup_deadline: Option<tokio::time::Instant>,
    ) -> Result<Self> {
        Self::bind_authorized_inner(
            paths,
            authority,
            startup_lock,
            startup_deadline,
            || Ok(()),
            || Ok(()),
            || ready(Ok(())),
        )
        .await
    }

    async fn bind_authorized_inner<C, V, B, Fut>(
        paths: &BrokerPaths,
        authority: Option<BrokerLaunchAuthority>,
        startup_lock: Option<BrokerInstanceLock>,
        startup_deadline: Option<tokio::time::Instant>,
        on_contention: C,
        after_verify: V,
        after_bind: B,
    ) -> Result<Self>
    where
        C: FnMut() -> Result<()>,
        V: FnOnce() -> Result<()>,
        B: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let lock = match startup_lock {
            Some(lock) => lock,
            None => BrokerInstanceLock::acquire(paths.instance_lock())?,
        };
        let publication = match authority {
            Some(authority) => {
                let deadline = startup_deadline.context(AUTHORIZED_DEADLINE_MISSING)?;
                let publication =
                    BrokerLaunchPublicationGuard::acquire_observed(paths, deadline, on_contention)
                        .await?;
                authority.verify(paths)?;
                after_verify()?;
                crate::remote_broker::deadline_fence::require_before(
                    deadline,
                    AUTHORIZED_DEADLINE_EXPIRED,
                )?;
                Some(publication)
            }
            None => {
                ensure!(startup_deadline.is_none(), UNAUTHORIZED_DEADLINE_PRESENT);
                after_verify()?;
                None
            }
        };
        let sockets = Self::bind_locked(
            paths,
            lock,
            || Ok(()),
            |_| {
                if let Some(deadline) = startup_deadline {
                    crate::remote_broker::deadline_fence::require_before(
                        deadline,
                        AUTHORIZED_DEADLINE_EXPIRED,
                    )?;
                }
                Ok(())
            },
            |_| Ok(()),
        )
        .await?;
        after_bind().await?;
        if let Some(deadline) = startup_deadline {
            crate::remote_broker::deadline_fence::require_before(
                deadline,
                AUTHORIZED_DEADLINE_EXPIRED,
            )?;
        }
        drop(publication);
        Ok(sockets)
    }

    #[cfg(test)]
    pub(in crate::remote_broker) async fn bind_startup_with_publication_hooks<C, V>(
        startup: crate::remote_broker::AuthorizedBrokerStartup,
        on_contention: C,
        after_verify: V,
    ) -> Result<Self>
    where
        C: FnMut() -> Result<()>,
        V: FnOnce() -> Result<()>,
    {
        let (paths, authority, lock, deadline) = startup.into_parts();
        Self::bind_authorized_inner(
            &paths,
            Some(authority),
            Some(lock),
            Some(deadline),
            on_contention,
            after_verify,
            || ready(Ok(())),
        )
        .await
    }

    #[cfg(test)]
    pub(in crate::remote_broker) async fn bind_startup_with_deadline_hook<B, Fut>(
        startup: crate::remote_broker::AuthorizedBrokerStartup,
        after_bind: B,
    ) -> Result<Self>
    where
        B: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let (paths, authority, lock, deadline) = startup.into_parts();
        Self::bind_authorized_inner(
            &paths,
            Some(authority),
            Some(lock),
            Some(deadline),
            || Ok(()),
            || Ok(()),
            after_bind,
        )
        .await
    }
}
