use super::{super::super::local_startup_support::*, support::*};
use std::{
    path::Path,
    process::{Command, Stdio},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use yolo_hub_client::Client;
use yolo_hub_state::HubState;
use yolo_runtime::process::ScopedWaitableChild;

async fn parent(command: &mut Command) -> (ScopedWaitableChild, ExactCleanup) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(command, deadline().into_std())
            .unwrap();
    let output = tokio::process::ChildStdout::from_std(child.take_stdout().unwrap()).unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout_at(
        deadline(),
        BufReader::new(output.take(MAX_PID_BYTES)).read_until(PID_SEPARATOR, &mut bytes),
    )
    .await
    .unwrap()
    .unwrap();
    let pid = serde_json::from_slice(&bytes).unwrap();
    (
        child,
        ExactCleanup(crate::ProcessIdentity::capture(pid).unwrap()),
    )
}

fn parent_command(service: &Path) -> Command {
    let executable = crate::bazel_runfile(&std::env::var(PARENT_ENV).unwrap()).unwrap();
    let mut command = Command::new(executable);
    command.arg(service);
    command
}

#[tokio::test(flavor = "multi_thread")]
async fn uncommitted_real_host_self_retires_after_its_launcher_is_force_killed() {
    let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let home = directory
        .path()
        .canonicalize()
        .unwrap()
        .join(HOST_DIRECTORY);
    let socket = yolo_runtime::paths::Paths::new(&home)
        .host_socket()
        .unwrap();
    let mut command = parent_command(&crate::runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .arg(HOST_SERVE)
        .arg(HOME)
        .arg(&home)
        .arg(SOCKET)
        .arg(&socket)
        .arg(yolo_startup_guard::ARGUMENT);
    let (launcher, service) = parent(&mut command).await;
    let client = yolo_client::Client::managed_local(&home, &socket).unwrap();
    let hold = tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(hold) = client.retain_host().await {
                break hold;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(hold.initial_host_pid(), service.0.pid);
    launcher
        .terminate_and_wait_reaped(deadline().into_std())
        .await
        .unwrap();
    // The authenticated connection is intentionally still open: idle shutdown
    // cannot explain the service exiting before this assertion completes.
    tokio::time::timeout(SETTLE_TIMEOUT, crate::wait_for_identity_gone(service.0))
        .await
        .unwrap();
    drop(hold);
}

#[tokio::test(flavor = "multi_thread")]
async fn uncommitted_real_hub_self_retires_after_its_launcher_is_force_killed() {
    let harness = crate::Harness::start().await;
    let _host_hold = retain_fixture_host(&harness).await;
    let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let state = directory
        .path()
        .canonicalize()
        .unwrap()
        .join(STATE_DIRECTORY);
    let mut command = parent_command(&hub_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Hub.arguments())
        .arg(STATE)
        .arg(&state)
        .arg(HOST_HOME)
        .arg(harness.home.path())
        .arg(READY);
    let (launcher, service) = parent(&mut command).await;
    let socket = HubState::socket_path_for(&state).unwrap();
    let client = tokio::time::timeout_at(deadline(), async {
        loop {
            assert!(service.0.matches(), "{HUB_READY_FAILURE}: {:?}", service.0);
            if let Ok(Some(hub)) = HubState::inspect_identity(&state, harness.home.path()) {
                if let Ok(client) = Client::connect_local(&socket, hub, deadline()).await {
                    break client;
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap_or_else(|error| {
        panic!(
            "{HUB_READY_FAILURE}: {error:?}; service={:?}; alive={}; host_alive={}; stderr={}",
            service.0,
            service.0.matches(),
            harness.host_identity().matches(),
            harness.host_stderr_tail()
        )
    });
    let stream = client.into_channel().unwrap().into_stream().unwrap();
    assert_eq!(
        u32::try_from(stream.get_ref().peer_cred().unwrap().pid().unwrap()).unwrap(),
        service.0.pid
    );
    launcher
        .terminate_and_wait_reaped(deadline().into_std())
        .await
        .unwrap();
    tokio::time::timeout(SETTLE_TIMEOUT, crate::wait_for_identity_gone(service.0))
        .await
        .unwrap();
    assert!(harness.client().status().await.is_ok());
}
