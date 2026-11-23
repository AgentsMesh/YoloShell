use super::super::protocol::CodexHookAck;
use super::ingress_live_source::{LiveProcessSource, PeerEvidence};
use crate::agent::hooks::HookEnvelope;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::Arc;
use yolo_codex_live_peer_fixture_contract as peer_contract;

const _: () =
    assert!(peer_contract::FRAME_HEADER_BYTES == super::super::constants::CODEX_FRAME_HEADER_BYTES);
const CLIENT_EXIT_ERROR: &str = "Codex ingress peer helper failed";

pub(super) struct PendingPeerExchange {
    child: Option<Child>,
    source: Arc<LiveProcessSource>,
}

impl PendingPeerExchange {
    pub(super) fn begin(
        source: Arc<LiveProcessSource>,
        executable: &Path,
        socket: &Path,
        event: &HookEnvelope,
        evidence: PeerEvidence,
    ) -> Self {
        let mut child = crate::process::spawn_waitable_test_child(command(executable, socket));
        let peer = crate::process::ProcessIdentity::capture(child.id()).unwrap();
        source.register_peer(peer, evidence);
        child.stdin.take().unwrap().write_all(&yolo_capnp_codec::encode(event).unwrap()).unwrap();
        Self {
            child: Some(child),
            source,
        }
    }

    pub(super) fn is_running(&mut self) -> bool {
        self.child.as_mut().unwrap().try_wait().unwrap().is_none()
    }

    pub(super) async fn finish(mut self) -> anyhow::Result<CodexHookAck> {
        let acknowledgement = self.observe().await?;
        self.wait_for_exit().await?;
        Ok(acknowledgement)
    }

    pub(super) async fn observe(&mut self) -> anyhow::Result<CodexHookAck> {
        let output = self
            .child
            .as_mut()
            .and_then(|child| child.stdout.take())
            .ok_or_else(|| anyhow::anyhow!(CLIENT_EXIT_ERROR))?;
        tokio::task::spawn_blocking(move || read_ack(output)).await?
    }

    pub(super) async fn wait_for_exit(mut self) -> anyhow::Result<()> {
        let mut child = self.child.take().unwrap();
        let status = tokio::task::spawn_blocking(move || child.wait()).await??;
        self.source.clear_peer();
        anyhow::ensure!(status.success(), CLIENT_EXIT_ERROR);
        Ok(())
    }
}

impl Drop for PendingPeerExchange {
    fn drop(&mut self) {
        self.source.clear_peer();
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            if let Ok(status) = child.wait() {
                yolo_coverage_test_support::discard_reaped_kill_profile(child.id(), status)
                    .unwrap();
            }
        }
    }
}

fn command(executable: &Path, socket: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .env(peer_contract::SOCKET_ENVIRONMENT, socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

fn read_ack(mut output: ChildStdout) -> anyhow::Result<CodexHookAck> {
    let mut header = [0_u8; peer_contract::FRAME_HEADER_BYTES];
    output.read_exact(&mut header)?;
    let length = usize::try_from(u32::from_be_bytes(header))?;
    anyhow::ensure!(
        length <= super::super::constants::CODEX_MAX_FRAME_BYTES,
        CLIENT_EXIT_ERROR
    );
    let mut payload = vec![0_u8; length];
    output.read_exact(&mut payload)?;
    Ok(yolo_capnp_codec::decode(&payload)?)
}
