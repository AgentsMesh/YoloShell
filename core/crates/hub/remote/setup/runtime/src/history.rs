use crate::RemoteSetupRuntime;
use uuid::Uuid;
use yolo_hub_dispatch::Operation;
use yolo_hub_remote_setup_protocol::{RemoteSetupHistory, SetupCheckpoint};
use yolo_hub_remote_setup_store::{SetupObservation, SetupPhase};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionStatus};

impl RemoteSetupRuntime {
    /// Reads only the durable journal. This intentionally does not consult a
    /// broker, launch a job, repair configuration, or reinterpret old success as
    /// present-day connectivity. It also works after a profile has been removed.
    pub(crate) fn inspect_history(&self, operation_id: Uuid) -> Result<Operation, PublicError> {
        if operation_id.is_nil() {
            return Err(PublicError::InvalidRequest);
        }
        self.engine
            .state
            .verify()
            .map_err(|_| PublicError::Unreachable)?;
        let observation = SetupObservation::observe(&self.engine.root, operation_id)
            .map_err(|_| PublicError::Unreachable)?;
        let checkpoint = if let Some(observation) = &observation {
            let record = observation.record();
            if record.owner != self.engine.state.peer_identity() {
                return Err(PublicError::Unauthorized);
            }
            let checkpoint = match &record.phase {
                SetupPhase::Prepared {} => SetupCheckpoint::Prepared {},
                SetupPhase::Provisioning { .. } => SetupCheckpoint::Provisioning {},
                SetupPhase::Routing { provision, .. } => SetupCheckpoint::Routing {
                    origin: origin(provision)?,
                },
                SetupPhase::Complete { provision, .. } => SetupCheckpoint::Complete {
                    origin: origin(provision)?,
                },
            };
            observation.verify().map_err(|_| PublicError::Unreachable)?;
            Some(checkpoint)
        } else {
            None
        };
        self.engine
            .root
            .verify_path()
            .map_err(|_| PublicError::Unreachable)?;
        self.engine
            .state
            .verify()
            .map_err(|_| PublicError::Unreachable)?;
        let history = RemoteSetupHistory::new(operation_id, self.engine.state.hub_id(), checkpoint)
            .map_err(|_| PublicError::Unreachable)?;
        Ok(Operation::Reply(Reply::RemoteSetupHistory { history }))
    }
}

fn origin(provision: &ProvisionStatus) -> Result<yolo_hub_identity::HubId, PublicError> {
    match &provision.fields().outcome {
        ProvisionOutcome::Ready { enrollment, .. } => Ok(enrollment.fields().peer.hub()),
        _ => Err(PublicError::Unreachable),
    }
}
