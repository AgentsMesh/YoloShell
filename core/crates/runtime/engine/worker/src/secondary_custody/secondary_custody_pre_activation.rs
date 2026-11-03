use super::WorkerCustodyLease;
use crate::pty_custody::{
    CustodyControl, CustodyControlAck, CustodyWorkerReadyDisposition, ExactCustodyReleasedAck,
};
use anyhow::{Context, Result};

const RELEASE_ACK_MISMATCH_DETAIL: &str =
    "Host PTY custody Released acknowledgement does not match its exact Release";
const ROLLBACK_AFTER_ACTIVATION_DETAIL: &str =
    "Worker cannot roll back PTY custody after activation";

impl WorkerCustodyLease {
    pub(crate) async fn rollback_before_activation(mut self) -> Result<ExactCustodyReleasedAck> {
        anyhow::ensure!(
            self.activation.take().is_some(),
            ROLLBACK_AFTER_ACTIVATION_DETAIL
        );
        let control = CustodyControl::release(&self.offer)?;
        let disposition = CustodyWorkerReadyDisposition::release(control.clone());
        self.stream.write(&disposition).await?;
        let acknowledgement: CustodyControlAck = self.stream.read().await?;
        acknowledgement
            .into_exact_released(&control)
            .context(RELEASE_ACK_MISMATCH_DETAIL)
    }
}
