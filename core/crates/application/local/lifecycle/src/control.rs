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

pub async fn restart(profile: &LocalProfile, executable: &Path) -> Result<StopReport> {
    if profile.state.exists() {
        validate_runtime_contract(&profile.state).context(STATE_INCOMPATIBLE)?;
    }
    yolo_hub_local_startup::require_host_custody()?;
    let report = stop(profile).await?;
    start(profile, executable)
        .await
        .context(STOPPED_NOT_STARTED)?;
    Ok(report)
}
