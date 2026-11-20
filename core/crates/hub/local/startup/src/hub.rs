use crate::{
    HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT, Launch, constants::*, fence::StartupFence, retry,
};
use anyhow::{Result, ensure};
use std::{
    os::fd::AsRawFd,
    path::Path,
    process::{Command, Stdio},
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
        .stderr(Stdio::null());
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
        child.terminate_and_wait_reaped(deadline.into_std()).await?;
        return wait_for_existing(socket, fence, deadline).await;
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
