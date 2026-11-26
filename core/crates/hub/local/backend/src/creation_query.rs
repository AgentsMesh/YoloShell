use crate::{LocalBackend, errors::map_error};
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_local_platform::runtime_request;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{
    CreateShellResult, CreationQueryOutcome, CreationQueryRequest, CreationQueryResult,
    PublicError, SecretCapability,
};
use yolo_terminal_model::{CreateTerminalResult, CreationQueryOutcome as RuntimeOutcome};

impl LocalBackend {
    pub(crate) async fn query_creation(
        &self,
        request: &CreationQueryRequest,
    ) -> Result<Reply, PublicError> {
        let request = runtime_request(request.creation());
        let query = self
            .client
            .query_creation(&request)
            .await
            .map_err(map_error)?;
        let outcome = match query.outcome {
            RuntimeOutcome::Missing {} => CreationQueryOutcome::Missing {},
            RuntimeOutcome::Pending {} => CreationQueryOutcome::Pending {},
            RuntimeOutcome::Ready { creation } => CreationQueryOutcome::Ready {
                creation: self.query_result(creation)?,
            },
            RuntimeOutcome::Finalized { creation } => CreationQueryOutcome::Finalized {
                creation: self.query_result(creation)?,
            },
            RuntimeOutcome::Survivor { creation } => CreationQueryOutcome::Survivor {
                creation: self.query_result(creation)?,
            },
            RuntimeOutcome::Archived { terminal_id } => CreationQueryOutcome::Archived {
                address: TerminalAddress {
                    hub_id: self.hub,
                    terminal_id: TerminalId::from_uuid(terminal_id)
                        .map_err(|_| PublicError::OutcomeUnknown)?,
                },
            },
        };
        let query = CreationQueryResult::new(query.creation_token, outcome)?;
        query.validate_for(request.creation_token, self.hub)?;
        Ok(Reply::CreationQuery { query })
    }

    fn query_result(&self, result: CreateTerminalResult) -> Result<CreateShellResult, PublicError> {
        Ok(CreateShellResult {
            address: TerminalAddress {
                hub_id: self.hub,
                terminal_id: TerminalId::from_uuid(result.terminal_id)
                    .map_err(|_| PublicError::OutcomeUnknown)?,
            },
            creation_capability: SecretCapability::from_uuid(result.creation_capability)
                .map_err(|_| PublicError::OutcomeUnknown)?,
        })
    }
}
