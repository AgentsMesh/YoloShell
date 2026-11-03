use crate::constants::*;
use anyhow::{Result, ensure};
use tokio::time::{Instant, sleep_until};

pub(crate) fn require_budget(deadline: Instant) -> Result<()> {
    ensure!(Instant::now() < deadline, DEADLINE_EXPIRED);
    Ok(())
}
pub(crate) fn probe_deadline(deadline: Instant) -> Instant {
    deadline.min(Instant::now() + PROBE_TIMEOUT)
}
pub(crate) async fn pause(deadline: Instant) -> Result<()> {
    require_budget(deadline)?;
    sleep_until(deadline.min(Instant::now() + POLL_INTERVAL)).await;
    require_budget(deadline)
}

// Startup probes only query Host status; an uncertain reply is safe to query again.
pub(crate) fn transport_error(error: &yolo_client::ClientError) -> bool {
    matches!(
        error,
        yolo_client::ClientError::Deadline
            | yolo_client::ClientError::Protocol(
                yolo_wire::ProtocolError::Io(_)
                    | yolo_wire::ProtocolError::EndOfStream
                    | yolo_wire::ProtocolError::OutcomeUnknown
            )
    )
}
