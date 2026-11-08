#[path = "attach_support/constants.rs"]
mod constants;
use super::support::{RunningHub, deadline};
use crate::relay_binary::status_fd::InheritedStatusChannel;
use constants::*;
pub use constants::{
    INPUT, MARKER, OBSERVE, REJECTED, REJECTED_MARKER, REPLACEMENT_INPUT, REPLACEMENT_MARKER,
    TAKEOVER,
};
use std::process::Stdio;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStdin, ChildStdout},
    time::timeout_at,
};
use yolo_hub_identity::{HubId, RuntimeTarget};
pub use yolo_hub_relay_control_protocol::{Status, StatusKind};
use yolo_runtime::local_unix::LocalUnixStream;

pub struct RunningAttach {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
    status: Option<LocalUnixStream>,
}
impl RunningAttach {
    pub fn start(
        hub: &RunningHub,
        target: RuntimeTarget,
        holder: uuid::Uuid,
        mode: Option<&str>,
    ) -> Self {
        Self::through(&hub.socket, hub.hub, target, holder, mode)
    }
    pub fn through(
        socket: &std::path::Path,
        entry: HubId,
        target: RuntimeTarget,
        holder: uuid::Uuid,
        mode: Option<&str>,
    ) -> Self {
        let binary = crate::bazel_runfile(&std::env::var(BINARY_ENV).unwrap()).unwrap();
        let mut command = tokio::process::Command::new(binary);
        command
            .args(yolo_launch_contract::ProcessRole::HubAttach.arguments())
            .arg(SOCKET)
            .arg(socket)
            .arg(HUB)
            .arg(entry.to_string())
            .arg(ORIGIN)
            .arg(target.address.hub_id.to_string())
            .arg(SESSION)
            .arg(target.address.terminal_id.to_string())
            .arg(EPOCH)
            .arg(target.runtime_epoch.to_string())
            .arg(HOLDER)
            .arg(holder.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let status = InheritedStatusChannel::prepare(&mut command);
        let mut child = command.spawn().unwrap();
        Self {
            input: child.stdin.take(),
            output: child.stdout.take().unwrap(),
            child,
            status: Some(status.into_peer()),
        }
    }
    pub async fn expect_status(&mut self, expected: StatusKind) -> Status {
        let status = timeout_at(
            deadline(),
            crate::read_relay_status(self.status.as_mut().unwrap()),
        )
        .await
        .unwrap();
        assert_eq!(status.kind, expected);
        status
    }
    pub async fn input(&mut self, bytes: &[u8]) {
        timeout_at(deadline(), self.input.as_mut().unwrap().write_all(bytes))
            .await
            .unwrap()
            .unwrap();
    }
    pub async fn marker(&mut self, marker: &str) {
        let mut seen = Vec::new();
        let mut buffer = [0; BUFFER_BYTES];
        let deadline = deadline();
        while !String::from_utf8_lossy(&seen).contains(marker) {
            let count = timeout_at(deadline, self.output.read(&mut buffer))
                .await
                .unwrap()
                .unwrap();
            assert_ne!(count, 0);
            seen.extend_from_slice(&buffer[..count]);
            assert!(seen.len() <= MAX_OBSERVED_BYTES);
        }
    }
    pub async fn stop(mut self, success: bool) {
        self.input.take();
        self.wait(success).await;
    }
    pub async fn lose_status(mut self) {
        self.status.take();
        self.wait(false).await;
    }
    async fn wait(mut self, success: bool) {
        match self.wait_for_exit().await {
            Ok(status) => assert_eq!(status.unwrap().success(), success),
            Err(_) => {
                let _ = self.child.start_kill();
                let _ = self.wait_for_exit().await;
                panic!("public attach helper did not settle");
            }
        }
    }

    async fn wait_for_exit(
        &mut self,
    ) -> Result<std::io::Result<std::process::ExitStatus>, tokio::time::error::Elapsed> {
        // Other owned spawns normalize SIGCHLD for waitable custody. Poll the
        // retained child so a replaced Tokio signal handler cannot delay reaping
        // until the outer deadline and expire unrelated Hub connections.
        timeout_at(deadline(), async {
            loop {
                if let Some(status) = self.child.try_wait()? {
                    return Ok(status);
                }
                tokio::time::sleep(EXIT_POLL_INTERVAL).await;
            }
        })
        .await
    }
}
