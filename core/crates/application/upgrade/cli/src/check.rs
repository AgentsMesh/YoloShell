use crate::{candidate, constants::*, github, probe};
use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;
use yolo_hub_identity::HubId;
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;

#[derive(Clone, Debug, Args)]
pub struct Options {
    #[arg(long, hide = true, value_name = "PATH")]
    pub socket: Option<PathBuf>,
    #[arg(long, hide = true, value_name = "HUB_ID")]
    pub hub: Option<HubId>,
    #[arg(long, hide = true, value_name = "PATH")]
    pub state: Option<PathBuf>,
    #[arg(long = SERVICE_UNIT_ARGUMENT, hide = true, value_name = "PATH")]
    pub service_units: Vec<PathBuf>,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
    /// Release channel to select.
    #[arg(long, value_enum, default_value = "stable")]
    pub channel: crate::github_release::Channel,
    /// Check the latest release without installing or restarting.
    #[arg(long, alias = CHECK_RELEASE_ARGUMENT, conflicts_with_all = ["install_release", "restart", "admit", "no_restart", "validate_state"])]
    pub check: bool,
    /// Admit the already installed image without checking releases.
    #[arg(long, conflicts_with_all = ["install_release", "restart", "check", "no_restart", "validate_state"])]
    pub admit: bool,
    /// Restart the local Host and Hub, then admit the installed image.
    #[arg(long, conflicts_with_all = ["install_release", "check", "admit", "no_restart", "validate_state"])]
    pub restart: bool,
    /// Install a newer release without restarting; only with the default update.
    #[arg(long, conflicts_with_all = ["install_release", "check", "admit", "restart", "validate_state"])]
    pub no_restart: bool,
    /// Legacy install-only mode.
    #[arg(long, hide = true, conflicts_with_all = ["check", "admit", "restart", "no_restart", "validate_state"])]
    pub install_release: bool,
    /// Validate the local Hub state contract with this image without changing it.
    #[arg(long, hide = true, conflicts_with_all = ["install_release", "restart", "check", "admit", "no_restart"])]
    pub validate_state: bool,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, value_name = "PATH", conflicts_with_all = [STATE_ARGUMENT, SOCKET_ARGUMENT, HUB_ARGUMENT])]
    pub local_profile: Option<PathBuf>,
    #[arg(long, hide = true)]
    pub repository: Option<String>,
}

pub fn run(options: Options) -> Result<()> {
    if options.check {
        return github::check(&options);
    }
    if options.install_release {
        let _ = github::install(&options)?;
        println!("run `yolo restart` to activate it");
        return Ok(());
    }
    let profile = match options.local_profile.as_deref() {
        Some(path) => LocalProfile::load(path)?,
        None => yolo_local_profile::current()?,
    };
    if options.validate_state {
        return yolo_local_lifecycle::validate_state(&profile);
    }
    let runtime = tokio::runtime::Runtime::new()?;
    if options.admit {
        return admit(&options, &profile, &runtime, None);
    }
    if options.restart {
        let executable = std::env::current_exe()?;
        yolo_local_lifecycle::validate_state(&profile)?;
        runtime.block_on(yolo_local_lifecycle::restart(&profile, &executable))?;
        println!("restarted local Host and Hub");
        return admit(&options, &profile, &runtime, None);
    }
    let installed = github::update(&options)?;
    let status = runtime.block_on(yolo_local_lifecycle::status(&profile))?;
    let running = status.hub.running || status.host.running;
    if !running {
        println!("no local services are running; run `yolo server start` to start them");
        return Ok(());
    }
    let restart_needed = installed.is_some() || status.restart_needed;
    if options.no_restart {
        if restart_needed {
            println!("run `yolo restart` to activate it");
        }
        return Ok(());
    }
    if restart_needed {
        let executable = installed.clone().unwrap_or(std::env::current_exe()?);
        match &installed {
            Some(installed) => candidate::validate_state(&options, installed)?,
            None => yolo_local_lifecycle::validate_state(&profile)?,
        }
        runtime.block_on(yolo_local_lifecycle::restart(&profile, &executable))?;
        println!("restarted local Host and Hub");
    }
    admit(&options, &profile, &runtime, installed)
}

fn admit(
    options: &Options,
    profile: &LocalProfile,
    runtime: &tokio::runtime::Runtime,
    installed: Option<PathBuf>,
) -> Result<()> {
    if let Some(installed) = installed {
        return candidate::admit(options, &installed);
    }
    let options = local_options(options.clone(), profile)?;
    let candidate_path = match installed {
        Some(link) => link.canonicalize()?,
        None => std::env::current_exe()?.canonicalize()?,
    };
    let candidate = yolo_local_lifecycle::image(&candidate_path)?;
    let report = runtime.block_on(probe::probe(&options, candidate))?;
    if options.json {
        println!("{}", report.to_json()?);
    } else {
        report.print();
    }
    anyhow::ensure!(report.ready(), "{}", UPGRADE_REFUSED);
    Ok(())
}

fn local_options(mut options: Options, profile: &LocalProfile) -> Result<Options> {
    let state = options
        .state
        .clone()
        .unwrap_or_else(|| profile.state.clone());
    let socket = options
        .socket
        .clone()
        .unwrap_or(HubState::socket_path_for(&state)?);
    let hub = options
        .hub
        .or_else(|| HubState::hub_id_for(&state, &profile.host_home).ok())
        .context(
            "local Hub identity is unavailable; run `yolo server start` to initialize the profile, or pass --local-profile",
        )?;
    let service_units = if options.service_units.is_empty() {
        probe::default_units()
    } else {
        options.service_units.clone()
    };
    options.socket = Some(socket);
    options.hub = Some(hub);
    options.state = Some(state);
    options.service_units = service_units;
    Ok(options)
}
