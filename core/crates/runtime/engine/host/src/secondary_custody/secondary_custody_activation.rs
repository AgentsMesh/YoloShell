use super::*;
use crate::pty_custody::{CustodyWorkerReadyDisposition, ExactCustodyWorkerActivation};
use std::sync::atomic::Ordering;

const READY_WRITE_CONTEXT: &str = "send PTY custody Ready acknowledgement";
const ACTIVATION_READ_CONTEXT: &str = "read Worker PTY custody activation";
const ACTIVATION_MISMATCH_CONTEXT: &str = "validate exact Worker PTY custody activation";
const READY_STREAM_MISSING_DETAIL: &str = "PTY custody Ready stream is unavailable";
const MONITOR_RECEIVER_MISSING_DETAIL: &str = "PTY custody monitor is already active";
const ACTIVATION_STATE_DETAIL: &str =
    "PTY custody lease cannot become active from its current state";
const PRE_ACTIVATION_ROLLBACK_DETAIL: &str = "Worker rolled back PTY custody before activation";
const RELEASED_DURING_ACTIVATION_DETAIL: &str =
    "Worker released PTY custody before Host availability publication";
const UNCERTAIN_RELEASE_DURING_ACTIVATION_DETAIL: &str =
    "Worker PTY custody release became uncertain during Host activation";
const ACTIVATION_PLATFORM_DETAIL: &str =
    "PTY custody platform bundle is unavailable during activation";
pub(super) enum WireActivationDisposition {
    Activated,
    RolledBack,
}

enum AvailabilityDisposition {
    Ready,
    Released,
    ReleaseUncertain,
}

impl SecondaryCustodyLease {
    pub(super) async fn establish_wire_activation(
        self: &Arc<Self>,
    ) -> Result<WireActivationDisposition, WorkerRpcError> {
        let mut slot = self.stream.lock().await;
        write_ready(self, &mut slot).await?;
        let disposition = read_ready_disposition(&mut slot).await?;
        match disposition {
            CustodyWorkerReadyDisposition::Activated { activation } => {
                let exact = activation
                    .into_exact(&self.offer, self.controller_generation)
                    .map_err(|error| {
                        let detail = super::containment::error_detail(error);
                        WorkerRpcError::identity_mismatch(format!(
                            "{ACTIVATION_MISMATCH_CONTEXT}: {detail}"
                        ))
                    })?;
                self.install_wire_activation(&mut slot, exact)?;
                Ok(WireActivationDisposition::Activated)
            }
            CustodyWorkerReadyDisposition::Release { control } => {
                let stream = stream_mut(&mut slot)?;
                super::rollback::before_activation(self, stream, control).await?;
                Ok(WireActivationDisposition::RolledBack)
            }
        }
    }

    fn install_wire_activation(
        self: &Arc<Self>,
        slot: &mut Option<CustodyUnixStream>,
        _proof: ExactCustodyWorkerActivation,
    ) -> Result<(), WorkerRpcError> {
        let mut state = super::super::lock_recover(&self.state);
        if *state != SecondaryCustodyState::Preparing {
            return Err(WorkerRpcError::protocol(ACTIVATION_STATE_DETAIL));
        }
        let receiver = super::super::lock_recover(&self.monitor_receiver)
            .take()
            .ok_or_else(|| WorkerRpcError::protocol(MONITOR_RECEIVER_MISSING_DETAIL))?;
        let stream = slot
            .take()
            .ok_or_else(|| WorkerRpcError::protocol(READY_STREAM_MISSING_DETAIL))?;
        *state = SecondaryCustodyState::Activating;
        self.publish_state(SecondaryCustodyState::Activating);
        self.wire_ready.store(true, Ordering::Release);
        drop(state);
        tokio::spawn(super::monitor::supervise(
            Arc::downgrade(self),
            stream,
            receiver,
        ));
        Ok(())
    }

    fn publish_availability(&self) -> Result<AvailabilityDisposition, WorkerRpcError> {
        let mut state = super::super::lock_recover(&self.state);
        match *state {
            SecondaryCustodyState::Activating => {
                if super::super::lock_recover(&self.platform).is_none() {
                    return Err(WorkerRpcError::protocol(ACTIVATION_PLATFORM_DETAIL));
                }
                self.available.store(true, Ordering::Release);
                *state = SecondaryCustodyState::Ready;
                self.publish_state(SecondaryCustodyState::Ready);
                Ok(AvailabilityDisposition::Ready)
            }
            SecondaryCustodyState::ReleaseAckPending | SecondaryCustodyState::Released => {
                Ok(AvailabilityDisposition::Released)
            }
            SecondaryCustodyState::ReleaseUncertain => {
                Ok(AvailabilityDisposition::ReleaseUncertain)
            }
            _ => Err(WorkerRpcError::protocol(ACTIVATION_STATE_DETAIL)),
        }
    }
}

impl Host {
    pub(super) async fn activate_secondary_custody(
        &self,
        successor: Arc<SecondaryCustodyLease>,
        fail_stop: &mut super::exchange_guard::PostPreparedFailStop,
    ) -> Result<(), WorkerRpcError> {
        match successor.establish_wire_activation().await? {
            WireActivationDisposition::Activated => {
                let predecessors = self.secondary_custody.predecessors(&successor.runtime);
                for predecessor in predecessors {
                    predecessor.supersede(&successor).await?;
                }
                let disposition = successor.publish_availability()?;
                fail_stop.disarm();
                match disposition {
                    AvailabilityDisposition::Ready => Ok(()),
                    AvailabilityDisposition::Released => {
                        Err(WorkerRpcError::transport(RELEASED_DURING_ACTIVATION_DETAIL))
                    }
                    AvailabilityDisposition::ReleaseUncertain => Err(WorkerRpcError::transport(
                        UNCERTAIN_RELEASE_DURING_ACTIVATION_DETAIL,
                    )),
                }
            }
            WireActivationDisposition::RolledBack => {
                fail_stop.disarm();
                Err(WorkerRpcError::transport(PRE_ACTIVATION_ROLLBACK_DETAIL))
            }
        }
    }
}

async fn write_ready(
    lease: &SecondaryCustodyLease,
    slot: &mut Option<CustodyUnixStream>,
) -> Result<(), WorkerRpcError> {
    let stream = stream_mut(slot)?;
    let ready =
        crate::pty_custody::CustodyHostAck::ready(&lease.offer, lease.controller_generation);
    stream
        .write(&ready)
        .await
        .map_err(|error| transport(READY_WRITE_CONTEXT, error))
}

async fn read_ready_disposition(
    slot: &mut Option<CustodyUnixStream>,
) -> Result<CustodyWorkerReadyDisposition, WorkerRpcError> {
    stream_mut(slot)?
        .read()
        .await
        .map_err(|error| transport(ACTIVATION_READ_CONTEXT, error))
}

fn stream_mut(
    slot: &mut Option<CustodyUnixStream>,
) -> Result<&mut CustodyUnixStream, WorkerRpcError> {
    slot.as_mut()
        .ok_or_else(|| WorkerRpcError::protocol(READY_STREAM_MISSING_DETAIL))
}

fn transport(context: &'static str, error: anyhow::Error) -> WorkerRpcError {
    let detail = super::containment::error_detail(error);
    WorkerRpcError::transport(format!("{context}: {detail}"))
}
