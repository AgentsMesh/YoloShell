//! Runs the installed image for decisions that only it can make.
use crate::check::Options;
use crate::constants::*;
use anyhow::{Context, Result};
use std::path::Path;

/// Admission after an install must run with the candidate image. The invoking
/// CLI may be an older protocol and would otherwise reject the Hub it just
/// started. The installed binary re-runs the same read-only checks with its own
/// protocol and state-contract constants.
pub(crate) fn admit(options: &Options, installed: &Path) -> Result<()> {
    let mut command = command(options, installed, ADMIT_FLAG)?;
    if options.json {
        command.arg(JSON_FLAG);
    }
    let status = command.status().context(INSTALLED_ADMISSION_FAILED)?;
    anyhow::ensure!(status.success(), "{}", UPGRADE_REFUSED);
    Ok(())
}

/// The installed image, not the invoking updater, decides whether the local
/// state contract is still readable. This runs before services are stopped so a
/// refused migration leaves the running Host and Hub untouched.
pub(crate) fn validate_state(options: &Options, installed: &Path) -> Result<()> {
    let mut command = command(options, installed, VALIDATE_STATE_FLAG)?;
    let status = command.status().context(INSTALLED_STATE_FAILED)?;
    anyhow::ensure!(status.success(), "{}", INSTALLED_STATE_REFUSED);
    Ok(())
}

fn command(options: &Options, installed: &Path, mode: &str) -> Result<std::process::Command> {
    let mut command = std::process::Command::new(installed.canonicalize()?);
    command.arg(COMMAND).arg(mode);
    if let Some(profile) = &options.local_profile {
        command.arg(LOCAL_PROFILE_FLAG).arg(profile);
    }
    if let Some(socket) = &options.socket {
        command.arg(SOCKET_FLAG).arg(socket);
    }
    if let Some(hub) = options.hub {
        command.arg(HUB_FLAG).arg(hub.to_string());
    }
    if let Some(state) = &options.state {
        command.arg(STATE_FLAG).arg(state);
    }
    for unit in &options.service_units {
        command.arg(SERVICE_UNIT_FLAG).arg(unit);
    }
    Ok(command)
}
