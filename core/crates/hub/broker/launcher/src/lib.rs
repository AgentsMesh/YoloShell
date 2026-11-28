mod constants;
mod result;
use anyhow::{Context, Result, ensure};
use constants::*;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
};
use tokio::time::{Instant, timeout_at};
use yolo_broker_hub_protocol::{BrokerHubConnectionBinding, BrokerHubProfile};
use yolo_broker_launch_authority::{
    AUTHORITY_STDIO_ARGUMENT, HubBrokerLaunchScope, START_HUB_COMMAND, issue,
};
use yolo_hub_ssh_profile_protocol::SshBrokerProfile;
use yolo_hub_state::HubState;
use yolo_process::ScopedWaitableChild;
use yolo_remote_broker::{BrokerClient, BrokerLaunchDeadline, BrokerPaths};
use zeroize::Zeroizing;

pub struct Launch {
    pub executable: PathBuf,
    pub owner: Arc<HubState>,
    pub host_home: PathBuf,
    pub registration: SshBrokerProfile,
}

pub async fn ensure(
    launch: Launch,
    verify_current: Arc<dyn Fn() -> Result<()> + Send + Sync>,
    credentials: Option<Zeroizing<Vec<u8>>>,
    deadline: Instant,
) -> Result<BrokerHubConnectionBinding> {
    ensure!(Instant::now() < deadline, DEADLINE);
    launch
        .registration
        .validate()
        .context(VALIDATE_REGISTRATION)?;
    verify_current().context(VERIFY_REGISTRATION)?;
    let source = launch
        .registration
        .launch
        .as_ref()
        .context(NO_LAUNCH_SOURCE)?;
    let state_directory = yolo_paths::directory::PrivateDirectory::open_or_create_strict(
        &launch.registration.broker_state,
    )
    .context(PREPARE_STATE)?;
    let paths = BrokerPaths::new(&launch.registration.broker_state).context(PREPARE_STATE)?;
    state_directory.verify_path().context(VERIFY_STATE)?;
    paths.verify_state().context(VERIFY_STATE)?;
    let scope = HubBrokerLaunchScope::capture(
        &launch.owner,
        &launch.host_home,
        launch.registration.profile.clone(),
        source.config_source.clone(),
        &paths,
        launch.registration.config_revision.clone(),
        BrokerLaunchDeadline::capture(deadline).context(CAPTURE_SCOPE)?,
    )
    .context(CAPTURE_SCOPE)?;
    ensure!(
        scope.config_sha256() == source.source_sha256,
        SOURCE_CHANGED
    );
    let verify = verify_current.clone();
    let (mut issuer, input) = issue(launch.owner.clone(), scope, credentials, move || verify())
        .context(ISSUE_AUTHORITY)?;
    let mut command = Command::new(&launch.executable);
    command
        .args(yolo_launch_contract::ProcessRole::Remote.arguments())
        .arg(START_HUB_COMMAND)
        .arg(AUTHORITY_STDIO_ARGUMENT)
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    verify_current().context(VERIFY_REGISTRATION)?;
    let child = ScopedWaitableChild::spawn_before_deadline(&mut command, deadline.into_std())
        .context(SPAWN_LAUNCHER)?;
    issuer.bind_child(&child).context(BIND_LAUNCHER)?;
    let ready = timeout_at(deadline, issuer.serve())
        .await
        .context(DEADLINE)?
        .context(AUTHORITY_HANDOFF)?;
    child
        .wait_for_clean_exit(deadline.into_std())
        .await
        .context(WAIT_LAUNCHER)?;
    verify_current().context(VERIFY_REGISTRATION)?;
    paths.verify_state().context(VERIFY_STATE)?;
    result::validate(&ready, &launch.registration).context(VALIDATE_READINESS)?;
    let profile = BrokerHubProfile {
        host_id: launch.registration.profile.profile_id().into(),
        profile_authority_sha256: launch.registration.profile.authority_sha256().into(),
        config_revision: launch.registration.config_revision,
    };
    let binding = BrokerClient::new(paths.management_socket())
        .prepare_hub_transport(profile, deadline)
        .await
        .context(PREPARE_TRANSPORT)?;
    ensure!(
        binding.broker_instance_id == ready.instance_id,
        WRONG_BROKER
    );
    verify_current().context(VERIFY_REGISTRATION)?;
    paths.verify_state().context(VERIFY_STATE)?;
    Ok(binding)
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
