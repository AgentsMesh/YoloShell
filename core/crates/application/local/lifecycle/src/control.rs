use crate::constants::*;
use crate::process::StopReport;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_local_startup::Launch;
use yolo_hub_state::{HubState, validate_runtime_contract};
use yolo_local_profile::LocalProfile;

pub async fn stop(profile: &LocalProfile) -> Result<StopReport> {
    let hub = crate::process::stop_socket(
        &HubState::socket_path_for(&profile.state)?,
        HUB_SERVICE,
        Instant::now() + STOP_TIMEOUT,
    )
    .await?;
    let host = crate::process::stop_socket(
        &yolo_paths::Paths::new(&profile.host_home).host_socket()?,
        HOST_SERVICE,
        Instant::now() + STOP_TIMEOUT,
    )
    .await?;
    Ok(StopReport {
        services: vec![hub, host],
    })
}

pub async fn start(profile: &LocalProfile, executable: &Path) -> Result<()> {
    let launch = Launch {
        state: profile.state.clone(),
        host_home: profile.host_home.clone(),
        host_executable: executable.to_path_buf(),
        hub_executable: executable.to_path_buf(),
    };
    yolo_hub_local_startup::ensure_running(launch, Instant::now() + START_TIMEOUT).await?;
    Ok(())
}

/// Checks the local state contract with the constants compiled into this
/// process. Callers that activate a different image must run this check with
/// that image instead; see `yolo_local_lifecycle::restart`.
pub fn validate_state(profile: &LocalProfile) -> Result<()> {
    if profile.state.exists() {
        validate_runtime_contract(&profile.state).context(STATE_INCOMPATIBLE)?;
    }
    Ok(())
}

/// Stops and starts the local services. The caller owns the state-contract
/// preflight because only the image that will run may decide compatibility: an
/// older updater must validate with its installed candidate, not itself.
pub async fn restart(profile: &LocalProfile, executable: &Path) -> Result<StopReport> {
    yolo_hub_local_startup::require_host_custody()?;
    let report = stop(profile).await?;
    start(profile, executable)
        .await
        .context(STOPPED_NOT_STARTED)?;
    Ok(report)
}
