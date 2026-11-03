use super::*;
use crate::pty_custody::{CustodyControl, CustodyHolderObservation};

const CONTROL_READ_CONTEXT: &str = "read PTY custody control frame";
const MONITOR_CANCELLED_DETAIL: &str =
    "PTY secondary-custody monitor was cancelled before terminal settlement";
const MONITOR_RECOVERY_PANIC_CONTEXT: &str = "PTY secondary-custody monitor recovery panicked";
const WORKER_GONE_DETAIL: &str = "PTY custody worker exited while its control stream remained open";
const GUARDIAN_GONE_DETAIL: &str = "detached PTY custody guardian is proven gone";
const GUARDIAN_INDETERMINATE_DETAIL: &str =
    "detached PTY custody guardian identity is indeterminate";
const WORKER_EXIT_CONTROL_DRAIN_TIMEOUT: std::time::Duration =
    crate::session_timing::PROCESS_CAPABILITY_EXIT_EVENT_CONVERGENCE_TIMEOUT;

pub(super) async fn supervise(
    lease: Weak<SecondaryCustodyLease>,
    stream: CustodyUnixStream,
    commands: tokio::sync::mpsc::Receiver<SecondaryCustodyCommand>,
) {
    let mut fail_closed = MonitorFailClosed::new(lease.clone());
    let recovery = lease.clone();
    let outcome =
        super::containment::catch_future_detail(supervise_inner(lease, stream, commands)).await;
    if let Err(detail) = outcome {
        super::failure::fail_closed(
            recovery,
            format!("{MONITOR_RECOVERY_PANIC_CONTEXT}: {detail}"),
        );
    }
    fail_closed.disarm();
}

async fn supervise_inner(
    lease: Weak<SecondaryCustodyLease>,
    stream: CustodyUnixStream,
    commands: tokio::sync::mpsc::Receiver<SecondaryCustodyCommand>,
) {
    let outcome =
        super::containment::catch_future_detail(run(lease.clone(), stream, commands)).await;
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(fault)) => super::failure::respond(lease, fault).await,
        Err(detail) => {
            super::failure::respond(lease, super::failure::CustodyMonitorFault::panic(detail)).await
        }
    }
}

struct MonitorFailClosed {
    lease: Weak<SecondaryCustodyLease>,
    armed: bool,
}

impl MonitorFailClosed {
    fn new(lease: Weak<SecondaryCustodyLease>) -> Self {
        Self { lease, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for MonitorFailClosed {
    fn drop(&mut self) {
        if self.armed {
            super::failure::fail_closed(self.lease.clone(), MONITOR_CANCELLED_DETAIL.to_owned());
        }
    }
}

async fn run(
    lease_ref: Weak<SecondaryCustodyLease>,
    mut stream: CustodyUnixStream,
    mut commands: tokio::sync::mpsc::Receiver<SecondaryCustodyCommand>,
) -> Result<(), super::failure::CustodyMonitorFault> {
    let worker_fault = worker_gone_fault(lease_ref.clone());
    tokio::pin!(worker_fault);
    let guardian_fault = guardian_fault(lease_ref.clone());
    tokio::pin!(guardian_fault);
    loop {
        tokio::select! {
            inbound = stream.read::<CustodyControl>() => {
                return handle_inbound(&lease_ref, &mut stream, inbound).await;
            }
            command = commands.recv() => {
                let Some(command) = command else { return Ok(()) };
                let Some(lease) = lease_ref.upgrade() else { return Ok(()) };
                return super::monitor_control::handle_command(&lease, &mut stream, command).await;
            }
            fault = &mut worker_fault => {
                let Err(worker_fault) = fault else { return Ok(()) };
                let deadline = tokio::time::sleep(WORKER_EXIT_CONTROL_DRAIN_TIMEOUT);
                tokio::pin!(deadline);
                let inbound = tokio::select! {
                    biased;
                    inbound = stream.read::<CustodyControl>() => Some(inbound),
                    fault = &mut guardian_fault => return fault,
                    _ = &mut deadline => None,
                };
                return match inbound {
                    Some(inbound) => handle_inbound(&lease_ref, &mut stream, inbound).await,
                    None => Err(worker_fault),
                }
            }
            fault = &mut guardian_fault => return fault,
        }
    }
}

async fn handle_inbound(
    lease_ref: &Weak<SecondaryCustodyLease>,
    stream: &mut CustodyUnixStream,
    inbound: anyhow::Result<CustodyControl>,
) -> Result<(), super::failure::CustodyMonitorFault> {
    let Some(lease) = lease_ref.upgrade() else {
        return Ok(());
    };
    match inbound {
        Ok(control) => super::monitor_control::handle_release(&lease, stream, control).await,
        Err(error) if is_transport_disconnect(&error) => Err(disconnected(error)),
        Err(error) => Err(failed(error)),
    }
}

async fn worker_gone_fault(
    lease_ref: Weak<SecondaryCustodyLease>,
) -> Result<(), super::failure::CustodyMonitorFault> {
    let mut interval = tokio::time::interval(super::super::PROCESS_EXIT_POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let Some(lease) = lease_ref.upgrade() else {
            return Ok(());
        };
        if super::failure::worker_is_proven_gone(&lease) {
            return Err(super::failure::CustodyMonitorFault::disconnected(
                WORKER_GONE_DETAIL,
            ));
        }
    }
}

pub(super) async fn guardian_fault(
    lease_ref: Weak<SecondaryCustodyLease>,
) -> Result<(), super::failure::CustodyMonitorFault> {
    let mut interval = tokio::time::interval(super::super::PROCESS_EXIT_POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let Some(lease) = lease_ref.upgrade() else {
            return Ok(());
        };
        if lease.guardian_monitor_retired() {
            return Ok(());
        }
        let Some(guardian) = lease.guardian else {
            continue;
        };
        let fault = match guardian.binding().process().observation() {
            CustodyHolderObservation::Current => continue,
            CustodyHolderObservation::ProvenGone => {
                super::failure::CustodyMonitorFault::guardian_gone(GUARDIAN_GONE_DETAIL)
            }
            CustodyHolderObservation::Indeterminate => {
                super::failure::CustodyMonitorFault::failed(GUARDIAN_INDETERMINATE_DETAIL)
            }
        };
        return Err(fault);
    }
}

fn disconnected(error: anyhow::Error) -> super::failure::CustodyMonitorFault {
    let detail = super::containment::error_detail(error);
    super::failure::CustodyMonitorFault::disconnected(format!("{CONTROL_READ_CONTEXT}: {detail}"))
}

fn failed(error: anyhow::Error) -> super::failure::CustodyMonitorFault {
    let detail = super::containment::error_detail(error);
    super::failure::CustodyMonitorFault::failed(format!("{CONTROL_READ_CONTEXT}: {detail}"))
}

pub(super) fn is_transport_disconnect(error: &anyhow::Error) -> bool {
    match error.downcast_ref::<crate::protocol::ProtocolError>() {
        Some(crate::protocol::ProtocolError::EndOfStream) => true,
        Some(crate::protocol::ProtocolError::Io(error)) => matches!(
            error.kind(),
            std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::NotConnected
        ),
        _ => false,
    }
}
