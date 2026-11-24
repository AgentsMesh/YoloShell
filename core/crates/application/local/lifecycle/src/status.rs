use crate::constants::*;
use crate::image;
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::time::{Instant, timeout_at};
use yolo_build_image::BuildImage;
use yolo_hub_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Reply};
use yolo_hub_state::{HubState, validate_runtime_contract};
use yolo_local_profile::LocalProfile;
use yolo_local_unix::LocalUnixStream;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StatusReport {
    pub client_version: String,
    pub client_image: BuildImage,
    pub hub: HubStatus,
    pub host: HostStatus,
    pub file_helper: Option<BuildImage>,
    pub live_images_consistent: bool,
    pub restart_needed: bool,
    pub state_contract: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HubStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub socket: PathBuf,
    pub reachable: bool,
    pub protocol: Option<u16>,
    pub expected_protocol: u16,
    pub image: Option<BuildImage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub socket: PathBuf,
    pub image: Option<BuildImage>,
}

pub async fn status(profile: &LocalProfile) -> Result<StatusReport> {
    let client_image = image(&std::env::current_exe()?.canonicalize()?)?;
    let hub_socket = HubState::socket_path_for(&profile.state)?;
    let host_socket = yolo_paths::Paths::new(&profile.host_home).host_socket()?;
    let mut hub_pid = None;
    let host_pid = crate::process::service_pid(&host_socket).await;
    let mut hub_reachable = false;
    let mut protocol = None;
    let mut service_build = None;
    if let Ok(hub_id) = HubState::hub_id_for(&profile.state, &profile.host_home) {
        // Each probe owns its own budget: resolving the identity and inspecting
        // the Host can consume most of a shared one before the socket is tried.
        match connect_hub(
            &hub_socket,
            hub_id,
            Instant::now() + STATUS_TIMEOUT,
            &mut protocol,
        )
        .await
        {
            Ok((Some(mut client), pid)) => {
                hub_pid = Some(pid);
                hub_reachable = true;
                if let Ok(Reply::Build { build }) = client
                    .call(
                        hub_id,
                        Command::GetBuild {},
                        Instant::now() + STATUS_TIMEOUT,
                    )
                    .await
                {
                    service_build = Some(build);
                }
            }
            Ok((None, pid)) => hub_pid = Some(pid),
            Err(_) => hub_pid = crate::process::service_pid(&hub_socket).await,
        }
    } else {
        hub_pid = crate::process::service_pid(&hub_socket).await;
    }
    let consistent = service_build
        .as_ref()
        .is_some_and(|build| build.all_match(&build.hub));
    let restart_needed = service_build
        .as_ref()
        .is_some_and(|build| !build.all_match(&client_image))
        || protocol.is_some_and(|version| version != HUB_PROTOCOL_VERSION);
    let (hub_image, host_image, file_helper) = service_build
        .map(|build| (Some(build.hub), Some(build.host), Some(build.file_helper)))
        .unwrap_or_default();
    Ok(StatusReport {
        client_version: env!("CARGO_PKG_VERSION").to_owned(),
        client_image,
        hub: HubStatus {
            running: hub_pid.is_some(),
            pid: hub_pid,
            socket: hub_socket,
            reachable: hub_reachable,
            protocol,
            expected_protocol: HUB_PROTOCOL_VERSION,
            image: hub_image,
        },
        host: HostStatus {
            running: host_pid.is_some(),
            pid: host_pid,
            socket: host_socket,
            image: host_image,
        },
        file_helper,
        live_images_consistent: consistent,
        restart_needed,
        state_contract: validate_runtime_contract(&profile.state).is_ok(),
    })
}

async fn connect_hub(
    path: &Path,
    hub_id: HubId,
    deadline: Instant,
    protocol: &mut Option<u16>,
) -> Result<(Option<Client<LocalUnixStream>>, u32)> {
    let stream = timeout_at(deadline, LocalUnixStream::connect(path))
        .await
        .context(CONNECT_TIMEOUT)?
        .context(CONNECT_FAILED)?;
    let credentials = stream.peer_cred().context(CONNECT_CREDENTIALS)?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    ensure!(
        credentials.uid() == unsafe { libc::geteuid() },
        CONNECT_UID_MISMATCH
    );
    let pid = credentials
        .pid()
        .and_then(|pid| u32::try_from(pid).ok())
        .context(CONNECT_PID_UNAVAILABLE)?;
    let client = Client::open_observing_version(stream, hub_id, deadline, |version| {
        *protocol = Some(version);
    })
    .await
    .ok();
    Ok((client, pid))
}
