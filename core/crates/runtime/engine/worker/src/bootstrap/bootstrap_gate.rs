use super::bootstrap_exec_observer::{BootstrapExecObserver, BootstrapExecTarget};
use super::constants::*;
use crate::local_unix::LocalUnixStream;
#[cfg(test)]
use crate::local_unix::LocalUnixStream as UnixStream;
use crate::protocol::{read_frame, write_frame};
use anyhow::{Context, Result, bail, ensure};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixListener;
use tokio::time::{Instant, timeout, timeout_at};

#[cfg(target_os = "macos")]
pub(crate) const BOOTSTRAP_GATE_CAPABILITY_UNPREPARED_DETAIL: &str =
    "bootstrap process capability watcher is not live before Prepared";
#[cfg(target_os = "macos")]
pub(crate) const BOOTSTRAP_GATE_CAPABILITY_UNRELEASABLE_DETAIL: &str =
    "bootstrap process capability watcher is not live before Go";
pub(crate) const BOOTSTRAP_GATE_WRITE_CLOSE_CONTEXT: &str = "close bootstrap gate write half";
pub(crate) const BOOTSTRAP_GATE_PEER_CREDENTIAL_CONTEXT: &str = "read bootstrap peer credentials";
pub(crate) const BOOTSTRAP_GATE_DISCONNECTED_PEER_ERRNO: i32 = libc::ENOTCONN;

#[path = "bootstrap_gate_protocol.rs"]
mod protocol;
pub(crate) use protocol::{
    BootstrapCapabilityAck, BootstrapChildSpec, BootstrapExecFailureFrame, BootstrapGateExpected,
    BootstrapGateReply, BootstrapHello,
};
#[cfg(target_os = "macos")]
#[path = "bootstrap_descriptor_macos.rs"]
pub(crate) mod descriptor;

#[cfg(target_os = "macos")]
pub(crate) type BootstrapDescriptor =
    std::sync::Arc<crate::process::MacosProcessCapabilityManagerReader>;
#[cfg(not(target_os = "macos"))]
pub(crate) type BootstrapDescriptor = ();

pub(crate) struct ValidatedBootstrapGate(
    LocalUnixStream,
    BootstrapExecObserver,
    BootstrapDescriptor,
);
pub(crate) struct PreparedBootstrapGate(
    LocalUnixStream,
    BootstrapExecObserver,
    BootstrapDescriptor,
);
pub(crate) struct ReleasedBootstrapGate(LocalUnixStream, BootstrapExecObserver);

#[path = "bootstrap_gate_confirmation.rs"]
mod confirmation;
pub(crate) use confirmation::{BootstrapExecConfirmation, ExplicitBootstrapExecFailure};

impl ValidatedBootstrapGate {
    #[cfg(target_os = "macos")]
    pub(crate) fn descriptor_capability(&self) -> BootstrapDescriptor {
        std::sync::Arc::clone(&self.2)
    }

    pub(crate) async fn prepare(mut self) -> Result<PreparedBootstrapGate> {
        #[cfg(target_os = "macos")]
        ensure_capability_live(&self.2, BOOTSTRAP_GATE_CAPABILITY_UNPREPARED_DETAIL)?;
        write_reply(&mut self.0, BootstrapGateReply::Prepared).await?;
        Ok(PreparedBootstrapGate(self.0, self.1, self.2))
    }
}

impl PreparedBootstrapGate {
    pub(crate) async fn release(mut self) -> Result<ReleasedBootstrapGate> {
        #[cfg(target_os = "macos")]
        ensure_capability_live(&self.2, BOOTSTRAP_GATE_CAPABILITY_UNRELEASABLE_DETAIL)?;
        write_reply(&mut self.0, BootstrapGateReply::Go).await?;
        self.0
            .shutdown()
            .await
            .context(BOOTSTRAP_GATE_WRITE_CLOSE_CONTEXT)?;
        Ok(ReleasedBootstrapGate(self.0, self.1))
    }
}

pub(crate) async fn write_reply(
    stream: &mut LocalUnixStream,
    reply: BootstrapGateReply,
) -> Result<()> {
    timeout(BOOTSTRAP_GATE_TIMEOUT, write_frame(stream, &reply))
        .await
        .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn ensure_capability_live(
    descriptor: &BootstrapDescriptor,
    detail: &'static str,
) -> Result<()> {
    ensure!(
        descriptor.liveness_observation()
            == Some(crate::process::MacosProcessCapabilityObservation::WriterPresent),
        detail
    );
    Ok(())
}

pub(crate) async fn accept_bootstrap_gate(
    listener: &UnixListener,
    expected: &BootstrapGateExpected,
) -> Result<ValidatedBootstrapGate> {
    let deadline = Instant::now() + BOOTSTRAP_GATE_TIMEOUT;
    loop {
        let (stream, _) = timeout_at(deadline, listener.accept())
            .await
            .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
        let mut stream = LocalUnixStream::from(stream);
        if !accepted_peer_matches(&stream, expected.child.pid)? {
            continue;
        }
        let hello: BootstrapHello = timeout_at(deadline, read_frame(&mut stream))
            .await
            .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
        ensure!(
            hello.authentication() == expected.authentication,
            BOOTSTRAP_GATE_HELLO_CONTEXT
        );
        let observer = BootstrapExecObserver::register(BootstrapExecTarget {
            process: expected.child,
            thread_id: hello.exec_thread_id,
        })?;
        #[cfg(target_os = "macos")]
        let descriptor = admit_descriptor(&mut stream, expected, deadline).await?;
        #[cfg(not(target_os = "macos"))]
        let descriptor = ();
        return Ok(ValidatedBootstrapGate(stream, observer, descriptor));
    }
}

#[cfg(target_os = "macos")]
pub(crate) async fn admit_descriptor(
    stream: &mut LocalUnixStream,
    expected: &BootstrapGateExpected,
    deadline: Instant,
) -> Result<BootstrapDescriptor> {
    let issuer = crate::process::MacosProcessCapabilityIssuer::create()?;
    descriptor::send_before(stream, &issuer, deadline).await?;
    let acknowledgement: BootstrapCapabilityAck = timeout_at(deadline, read_frame(stream))
        .await
        .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
    ensure!(
        peer_matches(stream, expected.child.pid)? && expected.child.matches(),
        BOOTSTRAP_GATE_PEER_CONTEXT
    );
    issuer
        .commit_acknowledgement(acknowledgement.reader_handle, acknowledgement.writer_handle)
        .context(BOOTSTRAP_GATE_CAPABILITY_CONTEXT)
}

pub(crate) fn peer_matches(stream: &LocalUnixStream, child_pid: u32) -> Result<bool> {
    let peer = stream
        .peer_cred()
        .context(BOOTSTRAP_GATE_PEER_CREDENTIAL_CONTEXT)?;
    peer_identity_matches(&peer, child_pid)
}

pub(crate) fn accepted_peer_matches(stream: &LocalUnixStream, child_pid: u32) -> Result<bool> {
    match stream.peer_cred() {
        Ok(peer) => peer_identity_matches(&peer, child_pid),
        Err(error) if error.raw_os_error() == Some(BOOTSTRAP_GATE_DISCONNECTED_PEER_ERRNO) => {
            Ok(false)
        }
        Err(error) => Err(error).context(BOOTSTRAP_GATE_PEER_CREDENTIAL_CONTEXT),
    }
}

pub(crate) fn peer_identity_matches(
    peer: &tokio::net::unix::UCred,
    child_pid: u32,
) -> Result<bool> {
    if peer.uid() != unsafe { libc::geteuid() } {
        return Ok(false);
    }
    let pid = peer.pid().context(BOOTSTRAP_GATE_PEER_CONTEXT)?;
    if pid <= 0 {
        bail!(BOOTSTRAP_GATE_PEER_CONTEXT);
    }
    Ok(pid as u32 == child_pid)
}

include!("bootstrap_gate_test_wiring.rs");
