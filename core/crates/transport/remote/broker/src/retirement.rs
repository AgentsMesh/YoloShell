use super::client::{BrokerClient, BrokerClientError};
use super::handoff::{BrokerReconcileGuard, BrokerRetirement};
use super::launch_authority::revoke_broker_launches;
use super::{BROKER_READY_POLL_INTERVAL, BrokerPaths, BrokerStatus};
use anyhow::{Result, bail};

pub(super) const CREDENTIAL_RETIREMENT_CAPABILITY: &str = "credential-retirement";
const RETIREMENT_HOST_MISMATCH: &str = "broker retirement host identity changed";
const RETIREMENT_UNSUPPORTED: &str = "broker does not support credential retirement";
const RETIREMENT_TIMEOUT: &str = "broker retirement deadline exceeded";

pub(super) fn validate_retirement_status(
    status: &BrokerStatus,
    expected_host_id: &str,
) -> Result<(), BrokerClientError> {
    if status.host_id != expected_host_id {
        return Err(BrokerClientError::Retirement(format!(
            "{RETIREMENT_HOST_MISMATCH}: expected {}, received {}",
            expected_host_id, status.host_id
        )));
    }
    if !status.supports(CREDENTIAL_RETIREMENT_CAPABILITY) {
        return Err(BrokerClientError::Retirement(RETIREMENT_UNSUPPORTED.into()));
    }
    Ok(())
}

pub async fn retire_broker(
    paths: &BrokerPaths,
    expected_host_id: &str,
    deadline: tokio::time::Instant,
) -> Result<()> {
    let guard = BrokerReconcileGuard::acquire(paths, deadline).await?;
    revoke_broker_launches(paths, &guard, deadline).await?;
    let client = BrokerClient::new(paths.management_socket());
    loop {
        guard.verify()?;
        let observed = BrokerRetirement::capture(&guard)?;
        if observed.has_management_endpoint() {
            if let Err(error) = observed
                .request_retirement(&client, expected_host_id, &guard, deadline)
                .await
            {
                if observed.retire_if_unlocked(paths, &guard).await? {
                    return Ok(());
                }
                return Err(error);
            }
            return await_retirement(observed, paths, &guard, deadline).await;
        }
        if observed.retire_if_unlocked(paths, &guard).await? {
            return Ok(());
        }
        wait_for_progress(deadline).await?;
    }
}

async fn await_retirement(
    observed: BrokerRetirement,
    paths: &BrokerPaths,
    guard: &BrokerReconcileGuard,
    deadline: tokio::time::Instant,
) -> Result<()> {
    loop {
        if observed.retire_if_unlocked(paths, guard).await? {
            return Ok(());
        }
        wait_for_progress(deadline).await?;
    }
}

async fn wait_for_progress(deadline: tokio::time::Instant) -> Result<()> {
    if tokio::time::Instant::now() >= deadline {
        bail!(RETIREMENT_TIMEOUT);
    }
    tokio::time::sleep(BROKER_READY_POLL_INTERVAL).await;
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/retirement_status_tests.rs"]
mod tests;
