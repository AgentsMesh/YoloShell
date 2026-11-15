use crate::{Launch, constants::*, fence::StartupFence, retry};
use anyhow::Result;
use std::{
    path::Path,
    process::{Command, Stdio},
};
use tokio::time::{Instant, timeout_at};
use yolo_process::ScopedWaitableChild;

pub(crate) struct ReadyHost {
    hold: yolo_client::HostHold,
    pending: Option<(yolo_startup_guard::Sender, u32)>,
}

impl ReadyHost {
    pub async fn commit(
        self,
        fence: &StartupFence,
        deadline: Instant,
    ) -> Result<yolo_client::HostHold> {
        fence.verify()?;
        if let Some((sender, pid)) = self.pending {
            sender.commit(pid, deadline).await?;
        }
        fence.verify()?;
        Ok(self.hold)
    }
}

pub(crate) async fn ensure(
    launch: &Launch,
    socket: &Path,
    fence: &StartupFence,
    deadline: Instant,
) -> Result<ReadyHost> {
    fence.verify()?;
    let client = yolo_client::Client::managed_local(&launch.host_home, socket)?;
    let observed = timeout_at(retry::probe_deadline(deadline), client.retain_host()).await;
    fence.verify()?;
    match observed {
        Ok(Ok(hold)) => {
            return Ok(ReadyHost {
                hold,
                pending: None,
            });
        }
        Ok(Err(error)) if !retry::transport_error(&error) => return Err(error.into()),
        _ => {}
    }
    retry::require_budget(deadline)?;
    #[cfg(target_os = "linux")]
    require_custody()?;
    let mut command = Command::new(&launch.host_executable);
    let (sender, input) = yolo_startup_guard::channel()?;
    let mut sender = Some(sender);
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .arg(HOST_SERVE_COMMAND)
        .arg(HOME_ARGUMENT)
        .arg(&launch.host_home)
        .arg(SOCKET_ARGUMENT)
        .arg(socket)
        .arg(yolo_startup_guard::ARGUMENT)
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    fence.verify()?;
    let mut child = Some(ScopedWaitableChild::spawn_detached_before_deadline(
        &mut command,
        deadline.into_std(),
    )?);
    loop {
        fence.verify()?;
        let observed = timeout_at(retry::probe_deadline(deadline), client.retain_host()).await;
        fence.verify()?;
        match observed {
            Ok(Ok(hold)) => {
                let mut pending = None;
                if let Some(candidate) = child.take() {
                    if candidate.process_id() == hold.initial_host_pid() {
                        candidate.preserve_in_background()?;
                        pending =
                            Some((sender.take().expect(READY_MISSING), hold.initial_host_pid()));
                    } else {
                        candidate
                            .terminate_and_wait_reaped(deadline.into_std())
                            .await?;
                    }
                }
                fence.verify()?;
                return Ok(ReadyHost { hold, pending });
            }
            Ok(Err(error)) if !retry::transport_error(&error) => return Err(error.into()),
            _ => {}
        }
        if let Some(candidate) = child.as_mut() {
            if candidate.try_wait()?.is_some() {
                candidate.retire_reaped()?;
                child = None;
                sender = None;
            }
        }
        retry::pause(deadline).await?;
    }
}

/// A new Host is the component that creates the PTY custody domain, so this
/// capability belongs here rather than at the client entry point: an already
/// running Host can be reused by a caller without delegated custody.
#[cfg(target_os = "linux")]
fn require_custody() -> Result<()> {
    require_custody_available(
        yolo_process::linux_containment::LinuxContainmentBroker::discover()?.is_some(),
    )
}

#[cfg(target_os = "linux")]
pub(crate) fn require_custody_available(available: bool) -> Result<()> {
    anyhow::ensure!(available, LINUX_CUSTODY_REQUIRED);
    Ok(())
}
