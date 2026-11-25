use anyhow::{Result, ensure};
use std::time::Duration;

pub(super) fn require_before(
    deadline: tokio::time::Instant,
    expiration: &'static str,
) -> Result<()> {
    ensure!(tokio::time::Instant::now() < deadline, expiration);
    Ok(())
}

pub(super) async fn wait_for_next_attempt(
    deadline: tokio::time::Instant,
    interval: Duration,
    expiration: &'static str,
) -> Result<()> {
    require_before(deadline, expiration)?;
    let wake = tokio::time::Instant::now()
        .checked_add(interval)
        .map(|candidate| candidate.min(deadline))
        .unwrap_or(deadline);
    tokio::time::sleep_until(wake).await;
    require_before(deadline, expiration)
}
