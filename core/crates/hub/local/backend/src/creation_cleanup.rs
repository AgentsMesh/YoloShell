use crate::{LocalBackend, errors::map_error, summary::summary};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{CleanupTerminationOutcome, CreateShellResult, PublicError};
use yolo_terminal_model::{CleanupTerminationOutcome as RuntimeOutcome, TerminalView};

impl LocalBackend {
    pub(crate) async fn cleanup_terminate(
        &self,
        creation: CreateShellResult,
        force: bool,
    ) -> Result<Reply, PublicError> {
        require_origin(self.hub, creation)?;
        let outcome = self
            .client
            .cleanup_terminate(
                creation.address.terminal_id.as_uuid(),
                creation.creation_capability.as_uuid(),
                force,
            )
            .await
            .map_err(map_error)?;
        Ok(termination_reply(creation, outcome))
    }

    pub(crate) async fn cleanup_archive(
        &self,
        creation: CreateShellResult,
    ) -> Result<Reply, PublicError> {
        require_origin(self.hub, creation)?;
        let view = self
            .client
            .cleanup_archive(
                creation.address.terminal_id.as_uuid(),
                creation.creation_capability.as_uuid(),
            )
            .await
            .map_err(map_error)?;
        match archive_reply(self.hub, creation, &view)? {
            Reply::Terminal { terminal } => Ok(Reply::Terminal {
                terminal: yolo_hub_local_platform::annotate_workspace(
                    self.platform.as_deref(),
                    terminal,
                )
                .await?,
            }),
            reply => Ok(reply),
        }
    }
}

fn require_origin(hub: HubId, creation: CreateShellResult) -> Result<(), PublicError> {
    if creation.address.hub_id == hub {
        Ok(())
    } else {
        Err(PublicError::OriginMismatch)
    }
}

pub(crate) fn termination_reply(creation: CreateShellResult, outcome: RuntimeOutcome) -> Reply {
    let outcome = match outcome {
        RuntimeOutcome::Terminated => CleanupTerminationOutcome::Terminated,
        RuntimeOutcome::Survivor => CleanupTerminationOutcome::Survivor,
    };
    Reply::CreationTerminated {
        address: creation.address,
        outcome,
    }
}

pub(crate) fn archive_reply(
    hub: HubId,
    creation: CreateShellResult,
    view: &TerminalView,
) -> Result<Reply, PublicError> {
    require_origin(hub, creation)?;
    // The mutation has already reached the runtime. An unusable receipt cannot
    // prove that cleanup did not execute and must not authorize automatic replay.
    let session = summary(hub, view).map_err(|_| PublicError::OutcomeUnknown)?;
    if session.fields().address != creation.address || !session.fields().archived {
        return Err(PublicError::OutcomeUnknown);
    }
    Ok(Reply::Terminal { terminal: session })
}
