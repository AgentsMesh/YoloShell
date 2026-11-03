use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use yolo_client::{Client, ClientError};
use yolo_runtime::cli::constants;
use yolo_runtime::paths::Paths;
use yolo_runtime::protocol::ProtocolError;

const HOST_START_TIMEOUT: Duration = Duration::from_secs(30);
const HOST_START_INTERVAL: Duration = Duration::from_millis(50);
const HOST_EARLY_EXIT_CONTEXT: &str = "detached host exited before its socket became ready";
const HOST_NOT_READY_CONTEXT: &str = "host socket did not become ready";
const HOST_STATUS_CONTEXT: &str = "inspect detached host process status";

#[path = "bridge_child.rs"]
mod bridge_child;
use bridge_child::{HostChildStatus, SpawnedHost};
#[path = "bridge_rpc.rs"]
mod bridge_rpc;
use bridge_rpc::forward;

pub(super) async fn run(home: PathBuf, socket: Option<PathBuf>) -> Result<()> {
    let paths = Paths::new(home);
    let namespace = crate::host_namespace::HostNamespace::retain(&paths)?;
    let socket = socket.map(Ok).unwrap_or_else(|| namespace.socket())?;
    ensure_host_running(&paths, &socket).await?;
    namespace.verify()?;
    let client = Client::managed_local(paths.home(), &socket)?;
    forward(&client, tokio::io::stdin(), tokio::io::stdout()).await
}

pub(super) async fn ensure_host_running(paths: &Paths, socket: &Path) -> Result<()> {
    let deadline = tokio::time::Instant::now() + HOST_START_TIMEOUT;
    let namespace = crate::host_namespace::HostNamespace::retain(paths)?;
    let spawn = SpawnedHost::spawn;
    #[cfg(test)]
    ensure_host_running_until(paths, socket, deadline, spawn, std::future::ready(())).await?;
    #[cfg(not(test))]
    ensure_host_running_until(paths, socket, deadline, spawn).await?;
    namespace.verify()
}

async fn ensure_host_running_until<C, F>(
    paths: &Paths,
    socket: &Path,
    deadline: tokio::time::Instant,
    spawn: F,
    #[cfg(test)] before_spawn_check: impl std::future::Future<Output = ()>,
) -> Result<()>
where
    C: HostChildStatus,
    F: FnOnce(&Path, &[OsString]) -> Result<C>,
{
    let client = Client::managed_local(paths.home(), socket)?;
    match tokio::time::timeout_at(deadline, client.status()).await {
        Ok(Ok(_)) => return Ok(()),
        Ok(Err(error)) if !is_transport_error(&error) => return Err(error.into()),
        Ok(Err(_)) | Err(_) => {}
    }
    require_spawn_budget(deadline, socket)?;
    let executable = std::env::current_exe()?;
    let mut args = yolo_runtime::cli::ProcessRole::Host.prepend([
        constants::SERVE_COMMAND.into(),
        constants::DETACHED_SESSION_ARGUMENT.into(),
        constants::HOME_ARGUMENT.into(),
        paths.home().as_os_str().to_owned(),
    ]);
    if socket != paths.host_socket()? {
        args.extend([
            constants::SOCKET_ARGUMENT.into(),
            socket.as_os_str().to_owned(),
        ]);
    }
    #[cfg(test)]
    before_spawn_check.await;
    require_spawn_budget(deadline, socket)?;
    let mut child = spawn(&executable, &args)?;
    wait_for_host(&client, socket, &mut child, deadline).await
}

async fn wait_for_host<C: HostChildStatus>(
    client: &Client,
    socket: &Path,
    child: &mut C,
    deadline: tokio::time::Instant,
) -> Result<()> {
    let mut early_exit = None;
    loop {
        match tokio::time::timeout_at(deadline, client.status()).await {
            Ok(Ok(status)) => {
                if child.process_id() == Some(status.host_pid) {
                    child.preserve();
                }
                return Ok(());
            }
            Ok(Err(error)) if !is_transport_error(&error) => return Err(error.into()),
            Ok(Err(_)) => {}
            Err(_) => break,
        }
        if early_exit.is_none() {
            early_exit = child.try_wait_status().context(HOST_STATUS_CONTEXT)?;
        }
        let Some(remaining) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
            break;
        };
        tokio::time::sleep(remaining.min(HOST_START_INTERVAL)).await;
    }
    if early_exit.is_none() {
        early_exit = child.try_wait_status().context(HOST_STATUS_CONTEXT)?;
    }
    if let Some(status) = early_exit {
        anyhow::bail!("{HOST_EARLY_EXIT_CONTEXT}: {status}");
    }
    Err(host_not_ready(socket))
}

fn host_not_ready(socket: &Path) -> anyhow::Error {
    anyhow::anyhow!("{HOST_NOT_READY_CONTEXT}: {}", socket.display())
}

fn require_spawn_budget(deadline: tokio::time::Instant, socket: &Path) -> Result<()> {
    (tokio::time::Instant::now() < deadline)
        .then_some(())
        .ok_or_else(|| host_not_ready(socket))
}

fn is_transport_error(error: &ClientError) -> bool {
    // Bootstrap only queried HOST_STATUS; retrying that read cannot duplicate an effect.
    matches!(
        error,
        ClientError::Deadline
            | ClientError::Protocol(
                ProtocolError::Io(_) | ProtocolError::EndOfStream | ProtocolError::OutcomeUnknown
            )
    )
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/bridge_ancillary.rs"]
mod ancillary_test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/bridge_bootstrap_tests.rs"]
mod bootstrap_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/bridge_deadline_tests.rs"]
mod deadline_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/bridge_tests.rs"]
mod tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/bridge_wait_failure_tests.rs"]
mod wait_failure_tests;
