use super::*;
use crate::pty_custody::{CustodyControl, CustodyControlAck};

const CONTROL_TIMEOUT: std::time::Duration =
    crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT;
const CONTROL_READ_CONTEXT: &str = "read PTY custody control frame";
const CONTROL_WRITE_CONTEXT: &str = "write PTY custody control frame";
const RELEASE_MISMATCH_DETAIL: &str = "PTY custody Release does not match its exact offer";
const RELEASE_SETTLEMENT_CONTEXT: &str = "prove exact PTY custody Release settlement";
#[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
const RELEASE_PROBE_CONTEXT: &str = "capture exact PTY custody release probe";
const SUPERSEDE_ACK_MISMATCH_DETAIL: &str =
    "PTY custody Supersede acknowledgement does not match its exact control";
const CONTROL_TIMEOUT_DETAIL: &str = "PTY custody control exchange timed out";
#[cfg(test)]
const INJECTED_MONITOR_PANIC_DETAIL: &str = "injected PTY custody monitor panic";

pub(super) async fn handle_release(
    lease: &Arc<SecondaryCustodyLease>,
    stream: &mut CustodyUnixStream,
    control: CustodyControl,
) -> Result<(), super::failure::CustodyMonitorFault> {
    let prepared = tokio::time::timeout(CONTROL_TIMEOUT, async {
        if !control.matches_release(&lease.offer) {
            return Err(WorkerRpcError::identity_mismatch(RELEASE_MISMATCH_DETAIL));
        }
        let settlement = super::settlement::wait_for_exact_release(lease, &control)
            .await
            .map_err(|error| transport(RELEASE_SETTLEMENT_CONTEXT, error))?;
        let acknowledgement = CustodyControlAck::released(&control)
            .ok_or_else(|| WorkerRpcError::protocol(RELEASE_MISMATCH_DETAIL))?;
        #[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
        let probe = lease
            .master_fd_custody_probe()
            .map_err(|error| transport(RELEASE_PROBE_CONTEXT, error))?;
        #[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
        crate::host::secondary_custody_release_test_hook::pause(
            lease.runtime.terminal_id,
            crate::host::secondary_custody_release_test_hook::CustodyReleaseBoundary::BeforePlatformDrop,
            probe.clone(),
        )
        .await
        .map_err(|error| {
            WorkerRpcError::transport(super::containment::error_detail(error))
        })?;
        let pending = lease.commit_active_release(settlement)?;
        #[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
        crate::host::secondary_custody_release_test_hook::pause(
            lease.runtime.terminal_id,
            crate::host::secondary_custody_release_test_hook::CustodyReleaseBoundary::AfterPlatformDrop,
            probe,
        )
        .await
        .map_err(|error| {
            WorkerRpcError::transport(super::containment::error_detail(error))
        })?;
        Ok((pending, acknowledgement))
    })
    .await
    .map_err(|_| WorkerRpcError::transport(CONTROL_TIMEOUT_DETAIL))
    .and_then(|result| result);
    let (pending, acknowledgement) = prepared.map_err(|error| {
        super::failure::CustodyMonitorFault::failed(super::containment::display_detail(&error))
    })?;
    let delivered = tokio::time::timeout(CONTROL_TIMEOUT, stream.write(&acknowledgement)).await;
    match delivered {
        Ok(Ok(())) => {
            pending.acknowledge();
            Ok(())
        }
        Ok(Err(error)) => {
            let failure = transport(CONTROL_WRITE_CONTEXT, error);
            settle_uncertain_ack(lease, pending, failure).await
        }
        Err(_) => {
            let failure = WorkerRpcError::transport(CONTROL_TIMEOUT_DETAIL);
            settle_uncertain_ack(lease, pending, failure).await
        }
    }
}

async fn settle_uncertain_ack(
    lease: &SecondaryCustodyLease,
    pending: super::pending_release::PendingReleaseAck,
    failure: WorkerRpcError,
) -> Result<(), super::failure::CustodyMonitorFault> {
    if super::failure::worker_is_proven_gone_after_disconnect(lease).await {
        pending.acknowledge();
        return Ok(());
    }
    let detail = super::containment::display_detail(&failure);
    pending.fail(detail.clone());
    Err(super::failure::CustodyMonitorFault::failed(detail))
}

struct CommandFailure {
    error: WorkerRpcError,
    disconnected: bool,
}

impl CommandFailure {
    fn failed(error: WorkerRpcError) -> Self {
        Self {
            error,
            disconnected: false,
        }
    }

    fn transport(context: &'static str, error: anyhow::Error) -> Self {
        let disconnected = super::monitor::is_transport_disconnect(&error);
        Self {
            error: transport(context, error),
            disconnected,
        }
    }

    fn fault(&self, detail: String) -> super::failure::CustodyMonitorFault {
        let fault = if self.disconnected {
            super::failure::CustodyMonitorFault::disconnected(detail)
        } else {
            super::failure::CustodyMonitorFault::failed(detail)
        };
        fault.for_supersede()
    }
}

fn command_timeout() -> CommandFailure {
    CommandFailure::failed(WorkerRpcError::transport(CONTROL_TIMEOUT_DETAIL))
}

fn command_protocol(error: WorkerRpcError) -> CommandFailure {
    CommandFailure::failed(error)
}

pub(super) async fn handle_command(
    lease: &Arc<SecondaryCustodyLease>,
    stream: &mut CustodyUnixStream,
    command: SecondaryCustodyCommand,
) -> Result<(), super::failure::CustodyMonitorFault> {
    let (successor, successor_generation, completion) = match command {
        SecondaryCustodyCommand::Supersede {
            successor,
            successor_generation,
            completion,
        } => (successor, successor_generation, completion),
        #[cfg(test)]
        SecondaryCustodyCommand::Panic => panic!("{INJECTED_MONITOR_PANIC_DETAIL}"),
    };
    let control = CustodyControl::supersede(&lease.offer, &successor, successor_generation);
    let result = tokio::time::timeout(CONTROL_TIMEOUT, async {
        stream
            .write(&control)
            .await
            .map_err(|error| CommandFailure::transport(CONTROL_WRITE_CONTEXT, error))?;
        let acknowledgement: CustodyControlAck = stream
            .read()
            .await
            .map_err(|error| CommandFailure::transport(CONTROL_READ_CONTEXT, error))?;
        let expected = CustodyControlAck::superseded(&control).ok_or_else(|| {
            command_protocol(WorkerRpcError::protocol(SUPERSEDE_ACK_MISMATCH_DETAIL))
        })?;
        if acknowledgement != expected {
            return Err(command_protocol(WorkerRpcError::identity_mismatch(
                SUPERSEDE_ACK_MISMATCH_DETAIL,
            )));
        }
        lease.commit_superseded().map_err(command_protocol)
    })
    .await
    .map_err(|_| command_timeout())
    .and_then(|result| result);
    match result {
        Ok(()) => {
            let _ = completion.send(Ok(()));
            Ok(())
        }
        Err(error) => {
            let detail = super::containment::display_detail(&error.error);
            let fault = error.fault(detail);
            let _ = completion.send(Err(error.error));
            Err(fault)
        }
    }
}

fn transport(context: &'static str, error: anyhow::Error) -> WorkerRpcError {
    let detail = super::containment::error_detail(error);
    WorkerRpcError::transport(format!("{context}: {detail}"))
}
