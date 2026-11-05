//! Peer transport through the user's local OpenSSH client.
//!
//! The SSH child's stdin/stdout carry the peer byte stream. A hidden
//! `internal hub --peer-stdio` process on the provider connects that stream to
//! the Hub's private peer socket. No TCP listener, local forward or broker
//! profile is involved; `~/.ssh/config`, the agent and `known_hosts` remain the
//! user's own trust configuration.
use crate::constants::*;
use crate::{PeerDialFuture, PeerDialer};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::io::{AsyncReadExt, join};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::oneshot;
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_peer_io::PeerIo;
use yolo_process::ScopedWaitableChild;

pub struct SystemSshDialer {
    program: PathBuf,
}

impl Default for SystemSshDialer {
    fn default() -> Self {
        Self::new(SSH_PROGRAM)
    }
}

impl SystemSshDialer {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }
}

impl PeerDialer for SystemSshDialer {
    fn dial<'a>(
        &'a self,
        _peer: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async move {
            address.validate()?;
            let target = address.ssh_target().context(SSH_TARGET_MISSING)?;
            ensure!(Instant::now() < deadline, DEADLINE_EXPIRED);
            let mut command = Command::new(&self.program);
            command
                .arg("-T")
                .arg("-o")
                .arg("BatchMode=yes")
                .arg("-o")
                .arg(format!("ConnectTimeout={SSH_CONNECT_TIMEOUT_SECONDS}"))
                .arg("-o")
                .arg(format!("ServerAliveInterval={SSH_ALIVE_INTERVAL_SECONDS}"))
                .arg("-o")
                .arg(format!("ServerAliveCountMax={SSH_ALIVE_COUNT_MAX}"))
                .arg(target)
                .arg(remote_command(address.socket()))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut child = ScopedWaitableChild::spawn_before_deadline(
                &mut command,
                deadline.into_std(),
            )
            .context(SSH_SPAWN_FAILED)?;
            let stdin = stdio(child.take_stdin(), ChildStdin::from_std)?;
            let stdout = stdio(child.take_stdout(), ChildStdout::from_std)?;
            let stderr = stdio(child.take_stderr(), ChildStderr::from_std)?;
            let (close, closed) = oneshot::channel();
            tokio::spawn(supervise(child, stderr, closed));
            Ok(PeerIo::new(join(stdout, stdin), move || {
                let _ = close.send(());
            }))
        })
    }
}

fn stdio<S, T>(
    value: Option<S>,
    convert: impl FnOnce(S) -> std::io::Result<T>,
) -> Result<T> {
    convert(value.ok_or_else(|| anyhow::anyhow!(SSH_STDIO_UNAVAILABLE))?)
        .context(SSH_STDIO_UNAVAILABLE)
}

fn remote_command(socket: &Path) -> String {
    let socket = shell_quote(socket);
    format!(
        "if command -v {REMOTE_YOLO} >/dev/null 2>&1; then exec {REMOTE_YOLO} \
         {PEER_STDIO_COMMAND} {PEER_STDIO_FLAG} {PEER_SOCKET_FLAG} {socket}; else exec \
         \"{REMOTE_YOLO_FALLBACK}\" {PEER_STDIO_COMMAND} {PEER_STDIO_FLAG} \
         {PEER_SOCKET_FLAG} {socket}; fi"
    )
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

async fn supervise(
    mut child: ScopedWaitableChild,
    stderr: ChildStderr,
    closed: oneshot::Receiver<()>,
) {
    let stderr = tokio::spawn(read_stderr(stderr));
    let _ = closed.await;
    let status = child
        .terminate_and_wait_status(std::time::Instant::now() + SHUTDOWN_TIMEOUT)
        .await;
    tracing::debug!(message = SSH_FAILED, ?status);
    let detail = stderr.await.unwrap_or_default();
    if !detail.is_empty() {
        tracing::warn!(message = SSH_FAILED, %detail);
    }
}

async fn read_stderr(mut stderr: ChildStderr) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 512];
    while bytes.len() < MAX_STDERR_BYTES {
        match stderr.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(read) => bytes.extend_from_slice(&chunk[..read]),
        }
    }
    String::from_utf8_lossy(&bytes).trim().to_owned()
}
