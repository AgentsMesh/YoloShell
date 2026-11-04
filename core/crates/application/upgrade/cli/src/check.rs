use crate::{
    constants::*,
    report::CheckReport,
    service::{self, ServiceUnit},
};
use anyhow::{Context, Result};
use clap::Args;
use std::{path::PathBuf, time::Duration};
use tokio::time::Instant;
use yolo_build_image::BuildImage;
use yolo_hub_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Reply};
use yolo_hub_state::{HubState, validate_runtime_contract};
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
    #[arg(long, hide = true)]
    pub(crate) repository: Option<String>,
}

pub(crate) fn run(options: Options) -> Result<()> {
    if options.check_release || options.install_release {
        return crate::github::run(&options);
    }
    let profile: LocalProfile = yolo_local_profile::current()?;
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
        default_units()
    } else {
        options.service_units.clone()
    };
    let mut options = options;
    options.socket = Some(socket);
    options.hub = Some(hub);
    options.state = Some(state);
    options.service_units = service_units;
    let candidate = service::image(&std::env::current_exe()?.canonicalize()?)?;
    let report = tokio::runtime::Runtime::new()?.block_on(probe(&options, candidate.clone()));
    let report = report?;
    if options.json {
        println!("{}", report.to_json()?);
    } else {
        print_report(&report);
    }
    anyhow::ensure!(
        report.ready(),
        "upgrade admission refused; stop the old Hub and runner after reviewing the failed checks"
    );
    Ok(())
}

async fn probe(options: &Options, candidate: BuildImage) -> Result<CheckReport> {
    let mut report = CheckReport::new(candidate.clone());
    report.push(
        CHECK_CANDIDATE,
        true,
        format!("{} bytes", candidate.size_bytes()),
    );
    let deadline = Instant::now() + Duration::from_secs(CHECK_DEADLINE_SECONDS);
    let client = Client::connect_local(
        options.socket.as_ref().unwrap(),
        options.hub.unwrap(),
        deadline,
    )
    .await;
    let mut client = match client {
        Ok(client) => client,
        Err(error) => {
            report.push(
                CHECK_PROTOCOL,
                false,
                format!("cannot contact running Hub: {error}"),
            );
            check_state_and_units(options, candidate, &mut report);
            return Ok(report);
        }
    };
    let hello = client.hello().clone();
    report.push(
        CHECK_PROTOCOL,
        hello.version == HUB_PROTOCOL_VERSION,
        format!(
            "running={}, expected={}",
            hello.version, HUB_PROTOCOL_VERSION
        ),
    );
    match client
        .call(options.hub.unwrap(), Command::GetBuild {}, deadline)
        .await
    {
        Ok(Reply::Build { build }) => {
            report.push(
                CHECK_BUILD,
                build.host == candidate && build.hub == candidate && build.file_helper == candidate,
                format!(
                    "host={}, hub={}, file_helper={}",
                    build.host.sha256(),
                    build.hub.sha256(),
                    build.file_helper.sha256()
                ),
            );
        }
        Ok(reply) => report.push(
            CHECK_BUILD,
            false,
            format!("unexpected Hub reply: {reply:?}"),
        ),
        Err(error) => report.push(
            CHECK_BUILD,
            false,
            format!("cannot read live service images: {error}"),
        ),
    }
    check_state_and_units(options, candidate, &mut report);
    Ok(report)
}

fn check_state_and_units(options: &Options, candidate: BuildImage, report: &mut CheckReport) {
    let state = validate_runtime_contract(options.state.as_ref().unwrap());
    report.push(
        CHECK_STATE,
        state.is_ok(),
        state
            .err()
            .map_or_else(|| CONTRACT_MATCH.into(), |error| error.to_string()),
    );
    if options.service_units.is_empty() {
        report.push(
            CHECK_SERVICE_UNITS,
            !cfg!(target_os = "linux"),
            NO_SERVICE_UNITS.to_owned(),
        );
    }
    for path in &options.service_units {
        let name = format!("{CHECK_SERVICE_PREFIX}{}", path.display());
        match ServiceUnit::read(path) {
            Ok(unit) => report.push(
                name,
                unit.image == candidate,
                format!("{}", unit.executable.display()),
            ),
            Err(error) => report.push(name, false, error.to_string()),
        }
    }
}

fn default_units() -> Vec<PathBuf> {
    [HOST_RUNNER_UNIT, HUB_UNIT]
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect()
}

fn print_report(report: &CheckReport) {
    println!("upgrade {}", report.status);
    for check in &report.checks {
        println!(
            "{}: {} ({})",
            if check.passed { "ok" } else { "fail" },
            check.name,
            check.detail
        );
    }
}
