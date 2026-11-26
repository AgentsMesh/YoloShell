use crate::{
    HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT, Launch, constants::*, fence::StartupFence, retry,
};
use anyhow::{Context, Result, anyhow, ensure};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::fd::AsRawFd,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    process::{Command, ExitStatus, Stdio},
};
use tokio::time::{Instant, timeout_at};
use yolo_hub_client::{Client, ClientError};
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_hub_terminal_io::Descriptor;
use yolo_process::ScopedWaitableChild;

pub(crate) async fn ensure(
    launch: &Launch,
    socket: &Path,
    host_socket: &Path,
    fence: &StartupFence,
    deadline: Instant,
) -> Result<()> {
    fence.verify()?;
    let identity = fence.peer.hub();
    let observed = Client::connect_local(socket, identity, retry::probe_deadline(deadline)).await;
    fence.verify()?;
    match observed {
        Ok(_) => return Ok(()),
        Err(ClientError::Transport(
            TransportError::Io(_) | TransportError::Deadline | TransportError::TruncatedFrame,
        )) => {}
        Err(error) => return Err(error.into()),
    }
    retry::require_budget(deadline)?;
    let stderr_path = launch.state.join(HUB_STARTUP_STDERR);
    let stderr = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&stderr_path)
        .context(STARTUP_STDERR_OPEN_FAILED)?;
    let mut command = Command::new(&launch.hub_executable);
    let (commit, input) = yolo_startup_guard::channel()?;
    command
        .args(yolo_launch_contract::ProcessRole::Hub.arguments())
        .arg(HUB_STATE_ARGUMENT)
        .arg(&launch.state)
        .arg(HOST_HOME_ARGUMENT)
        .arg(&launch.host_home)
        .arg(HOST_SOCKET_ARGUMENT)
        .arg(host_socket)
        .arg(READY_ARGUMENT)
        .stdin(Stdio::from(input))
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr));
    fence.verify()?;
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(&mut command, deadline.into_std())?;
    let stdout = child
        .take_stdout()
        .ok_or_else(|| anyhow::anyhow!(READY_MISSING))?;
    let mut input = Descriptor::duplicate(stdout.as_raw_fd())?;
    let ready = timeout_at(deadline, crate::readiness::read_ready(&mut input)).await??;
    fence.verify()?;
    let Some(ready) = ready else {
        // A Hub that loses the singleton exits without readiness. Give the
        // winner a short grace to become reachable, then report why ours died.
        let status = child.terminate_and_wait_status(deadline.into_std()).await?;
        let grace = deadline.min(Instant::now() + EXISTING_HUB_GRACE);
        if wait_for_existing(socket, fence, grace).await.is_ok() {
            return Ok(());
        }
        return Err(startup_failure(status, &stderr_path, launch, host_socket).await);
    };
    ensure!(
        ready.schema_version == CONFIG_SCHEMA_VERSION
            && ready.entry_hub == identity
            && ready.socket_path == socket,
        READY_INVALID
    );
    Client::connect_local(socket, identity, deadline).await?;
    fence.verify()?;
    let pid = child.process_id();
    child.preserve_in_background()?;
    commit.commit(pid, deadline).await?;
    fence.verify()?;
    Ok(())
}

async fn wait_for_existing(socket: &Path, fence: &StartupFence, deadline: Instant) -> Result<()> {
    loop {
        fence.verify()?;
        let observed =
            Client::connect_local(socket, fence.peer.hub(), retry::probe_deadline(deadline)).await;
        fence.verify()?;
        match observed {
            Ok(_) => return Ok(()),
            Err(ClientError::Transport(
                TransportError::Io(_) | TransportError::Deadline | TransportError::TruncatedFrame,
            )) => {}
            Err(error) => return Err(error.into()),
        }
        retry::pause(deadline).await?;
    }
}

/// Startup errors are written to a bounded, private log before the Hub exits,
/// because its stderr cannot stay attached to a detached service.
async fn startup_failure(
    status: ExitStatus,
    stderr_path: &Path,
    launch: &Launch,
    host_socket: &Path,
) -> anyhow::Error {
    let mut message = format!("{HUB_EXITED_BEFORE_READY}: {status}");
    if let Some(detail) = startup_stderr(stderr_path) {
        message.push_str(": ");
        message.push_str(&detail);
    }
    if let Some(hint) = host_image_hint(launch, host_socket).await {
        message.push_str("; ");
        message.push_str(&hint);
    }
    anyhow!(message)
}

pub(crate) fn startup_stderr(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(MAX_STARTUP_STDERR_BYTES as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let detail = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    (!detail.is_empty()).then_some(detail)
}

async fn host_image_hint(launch: &Launch, host_socket: &Path) -> Option<String> {
    let client = yolo_client::Client::managed_local(&launch.host_home, host_socket).ok()?;
    let image = timeout_at(Instant::now() + PROBE_TIMEOUT, client.build_image())
        .await
        .ok()?
        .ok()?;
    Some(format!(
        "{HOST_IMAGE_PREFIX}{}{HOST_IMAGE_SUFFIX}",
        image.sha256()
    ))
}
