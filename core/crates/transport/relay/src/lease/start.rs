use super::{LeaseController, LeaseIdentity};
use crate::relay::status::RelayStatusSink;
use crate::relay::{RelayCancellation, RelayError, RelayOptions, RelayRuntime};
use crate::worker::RuntimeRef;
use tokio::time::Instant;
use uuid::Uuid;

impl LeaseController {
    pub(crate) fn pending(
        runtime: RelayRuntime,
        options: &RelayOptions,
        target: RuntimeRef,
        status: RelayStatusSink,
    ) -> Self {
        Self {
            runtime,
            target,
            identity: LeaseIdentity {
                holder_id: options.holder_id.clone(),
                controller_instance_id: Uuid::new_v4().to_string(),
            },
            writer_capability: crate::model::WriterCapability::generate(),
            lease: None,
            conflict_holder: None,
            retry_mode: super::LeaseAcquireMode::Observe,
            prepared_generation: None,
            status_announced: false,
            next_maintenance: Instant::now(),
            status,
            ownership_uncertain: false,
            control_lost: false,
            lifecycle: super::SessionLifecycle::Active,
        }
    }

    pub(crate) async fn start(
        &mut self,
        take_control: bool,
        cancellation: &RelayCancellation,
    ) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        self.retry_mode = if take_control {
            super::LeaseAcquireMode::Takeover
        } else {
            super::LeaseAcquireMode::Observe
        };
        cancellation
            .wait(self.retry_acquisition_recovering())
            .await?
    }
}
