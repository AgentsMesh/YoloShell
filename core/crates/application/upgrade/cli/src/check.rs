use crate::{constants::*, github, probe};
use anyhow::{Context, Result, bail};
use clap::Args;
use std::path::PathBuf;
use yolo_hub_identity::HubId;
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;

#[derive(Debug, Args)]
pub(crate) struct Options {
    #[arg(long, hide = true, value_name = "PATH")]
    pub(crate) socket: Option<PathBuf>,
    #[arg(long, hide = true, value_name = "HUB_ID")]
    pub(crate) hub: Option<HubId>,
    #[arg(long, hide = true, value_name = "PATH")]
    pub(crate) state: Option<PathBuf>,
    #[arg(long = SERVICE_UNIT_ARGUMENT, hide = true, value_name = "PATH")]
    pub(crate) service_units: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long, value_enum, default_value = "stable")]
    pub(crate) channel: crate::github_release::Channel,
    #[arg(long = CHECK_RELEASE_ARGUMENT)]
    pub(crate) check_release: bool,
    #[arg(long = INSTALL_RELEASE_ARGUMENT)]
    pub(crate) install_release: bool,
    /// Restart the local Host and Hub after the requested operation.
    #[arg(long)]
    pub(crate) restart: bool,
    #[arg(long, hide = true)]
    pub(crate) repository: Option<String>,
}

pub(crate) fn run(options: Options) -> Result<()> {
    if options.restart && options.check_release {
        bail!("{}", RESTART_CHECK_CONFLICT);
    }
    let profile: LocalProfile = yolo_local_profile::current()?;
    let installed = if options.check_release || options.install_release {
        let link = github::run(&options)?;
        if !options.restart {
            return Ok(());
        }
        link
    } else {
        None
    };
    let runtime = tokio::runtime::Runtime::new()?;
    if options.restart {
        let executable = installed.clone().unwrap_or(std::env::current_exe()?);
        runtime.block_on(yolo_local_lifecycle::restart(&profile, &executable))?;
        println!("restarted local Host and Hub");
    }
    let options = local_options(options, &profile)?;
    let candidate_path = match &installed {
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
        .context("local Hub identity is unavailable; update admission cannot select a Hub")?;
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
