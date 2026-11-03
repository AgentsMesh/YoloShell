use crate::errors::map_error;
use uuid::Uuid;
use yolo_client::Client;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::PublicError;
use yolo_terminal_model::{CreateTerminal, CreateTerminalResult, CreationQueryOutcome};

/// Create a shell, deliver its bounded input, and finalize the creation. The
/// creation token is the caller's idempotency identity, so a retry whose first
/// reply was lost recovers durable finalized evidence instead of treating the
/// retired token as a conflict.
pub(crate) async fn create_finalized(
    client: &Client,
    request: &CreateTerminal,
) -> Result<CreateTerminalResult, PublicError> {
    match client.create_session(request).await.map_err(map_error) {
        Ok(created) => {
            finalize(client, request.creation_token, created).await?;
            Ok(created)
        }
        Err(error @ (PublicError::Conflict | PublicError::OutcomeUnknown)) => {
            recover(client, request, error).await
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn address(hub: HubId, terminal_id: Uuid) -> Result<TerminalAddress, PublicError> {
    let terminal_id = TerminalId::from_uuid(terminal_id).map_err(|_| PublicError::Internal)?;
    Ok(TerminalAddress {
        hub_id: hub,
        terminal_id,
    })
}

async fn recover(
    client: &Client,
    request: &CreateTerminal,
    error: PublicError,
) -> Result<CreateTerminalResult, PublicError> {
    let outcome = client.query_creation(request).await.map_err(|_| error)?;
    match outcome.outcome {
        CreationQueryOutcome::Finalized { creation } => Ok(creation),
        CreationQueryOutcome::Ready { creation } => {
            finalize(client, request.creation_token, creation).await?;
            Ok(creation)
        }
        _ => Err(error),
    }
}

async fn finalize(
    client: &Client,
    creation_token: Uuid,
    created: CreateTerminalResult,
) -> Result<(), PublicError> {
    client
        .finalize_creation(
            creation_token,
            created.terminal_id,
            created.creation_capability,
        )
        .await
        .map_err(map_error)
}
