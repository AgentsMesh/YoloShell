use super::publication_gate::BrokerLaunchPublicationGuard;
use super::{BrokerPaths, BrokerReconcileGuard};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::future::{Future, ready};
use uuid::Uuid;

pub(super) const BROKER_LAUNCH_AUTHORITY_FILE: &str = "launch-authority.json";
const BROKER_LAUNCH_AUTHORITY_VERSION: u16 = 2;
const AUTHORITY_MISMATCH: &str = "broker launch authority was revoked";
const AUTHORITY_VERSION_UNSUPPORTED: &str = "unsupported broker launch authority version";
const AUTHORITY_READ_CONTEXT: &str = "read broker launch authority";
const AUTHORITY_PUBLISH_CONTEXT: &str = "publish broker launch authority";
const AUTHORITY_DEADLINE_EXPIRED: &str = "broker launch authority deadline expired";
const AUTHORITY_BOOT_MISMATCH: &str = "broker launch authority belongs to another boot";

#[path = "authorized_startup.rs"]
mod authorized_startup;
#[path = "launch_authority_format.rs"]
mod format;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrokerLaunchAuthority(Uuid);

pub struct AuthorizedBrokerStartup {
    pub(super) paths: BrokerPaths,
    pub(super) authority: BrokerLaunchAuthority,
    pub(super) lock: super::instance_lock::BrokerInstanceLock,
    pub(super) deadline: tokio::time::Instant,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthorityRecord {
    version: u16,
    token: Uuid,
    boot_id: String,
}

impl BrokerLaunchAuthority {
    pub(super) async fn issue(
        paths: &BrokerPaths,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
    ) -> Result<Self> {
        Self::issue_observed(
            paths,
            guard,
            deadline,
            || Ok(()),
            || ready(Ok(())),
            || Ok(()),
        )
        .await
    }

    async fn issue_observed<C, B, Fut, P>(
        paths: &BrokerPaths,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
        on_contention: C,
        before_publish: B,
        after_publish: P,
    ) -> Result<Self>
    where
        C: FnMut() -> Result<()>,
        B: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
        P: FnOnce() -> Result<()>,
    {
        guard.verify()?;
        let publication =
            BrokerLaunchPublicationGuard::acquire_observed(paths, deadline, on_contention).await?;
        guard.verify()?;
        let authority = Self(Uuid::new_v4());
        before_publish().await?;
        super::deadline_fence::require_before(deadline, AUTHORITY_DEADLINE_EXPIRED)?;
        authority.publish(paths)?;
        guard.verify()?;
        after_publish()?;
        super::deadline_fence::require_before(deadline, AUTHORITY_DEADLINE_EXPIRED)?;
        drop(publication);
        Ok(authority)
    }

    pub(super) async fn revoke(
        paths: &BrokerPaths,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
    ) -> Result<()> {
        Self::issue(paths, guard, deadline).await.map(|_| ())
    }

    #[cfg(test)]
    pub(super) async fn revoke_observed<C, P>(
        paths: &BrokerPaths,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
        on_contention: C,
        after_publish: P,
    ) -> Result<()>
    where
        C: FnMut() -> Result<()>,
        P: FnOnce() -> Result<()>,
    {
        Self::issue_observed(
            paths,
            guard,
            deadline,
            on_contention,
            || ready(Ok(())),
            after_publish,
        )
        .await
        .map(|_| ())
    }

    #[cfg(test)]
    pub(super) async fn issue_with_deadline_hook<B, Fut>(
        paths: &BrokerPaths,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
        before_publish: B,
    ) -> Result<Self>
    where
        B: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        Self::issue_observed(paths, guard, deadline, || Ok(()), before_publish, || Ok(())).await
    }

    pub fn verify(self, paths: &BrokerPaths) -> Result<()> {
        paths.verify_state()?;
        let record: AuthorityRecord =
            crate::paths::read_json(&paths.launch_authority()).context(AUTHORITY_READ_CONTEXT)?;
        ensure!(
            record.version == BROKER_LAUNCH_AUTHORITY_VERSION,
            AUTHORITY_VERSION_UNSUPPORTED
        );
        ensure!(record.token == self.0, AUTHORITY_MISMATCH);
        ensure!(
            crate::process::boot_identity_relation(&record.boot_id, &crate::process::boot_id())
                == crate::process::BootIdentityRelation::Same,
            AUTHORITY_BOOT_MISMATCH
        );
        paths.verify_state()
    }

    fn publish(self, paths: &BrokerPaths) -> Result<()> {
        let boot_id = crate::process::boot_id();
        ensure!(
            crate::process::boot_identity_relation(&boot_id, &boot_id)
                == crate::process::BootIdentityRelation::Same,
            AUTHORITY_BOOT_MISMATCH
        );
        let record = AuthorityRecord {
            version: BROKER_LAUNCH_AUTHORITY_VERSION,
            token: self.0,
            boot_id,
        };
        crate::paths::atomic_write_json(&paths.launch_authority(), &record)
            .context(AUTHORITY_PUBLISH_CONTEXT)
    }
}

pub async fn authorize_broker_launch(
    paths: &BrokerPaths,
    guard: &BrokerReconcileGuard,
    deadline: tokio::time::Instant,
) -> Result<BrokerLaunchAuthority> {
    BrokerLaunchAuthority::issue(paths, guard, deadline).await
}

pub(super) async fn revoke_broker_launches(
    paths: &BrokerPaths,
    guard: &BrokerReconcileGuard,
    deadline: tokio::time::Instant,
) -> Result<()> {
    BrokerLaunchAuthority::revoke(paths, guard, deadline).await
}
