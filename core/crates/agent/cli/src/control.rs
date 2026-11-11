use crate::constants::*;
use crate::receipt::Execution;
use crate::reject::rejected;
use anyhow::{Result, bail, ensure};
use tokio::time::Instant;
use uuid::Uuid;
use yolo_agent_cli_readiness::readiness_message;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    AgentSnapshot, InteractionResponse, Prompt,
};
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    LeaseGrant, LeaseMode, LeaseRequest, PublicError, WriterPrincipal,
};

pub async fn acquire(
    client: &mut LocalClient,
    target: RuntimeTarget,
    take_control: bool,
) -> Result<LeaseGrant> {
    let principal = WriterPrincipal {
        client_id: Uuid::new_v4(),
        controller_instance_id: Uuid::new_v4(),
    };
    let mode = if take_control {
        observed_mode(client, target).await?
    } else {
        LeaseMode::Acquire {}
    };
    let request = LeaseRequest {
        request_id: Uuid::new_v4(),
        target,
        principal,
        mode,
    };
    let deadline = Instant::now() + RPC_TIMEOUT;
    match client
        .call(target.address.hub_id, Command::Lease { request }, deadline)
        .await
    {
        Ok(Reply::Lease { grant }) => {
            ensure!(
                grant.fields().target == target && grant.fields().principal == principal,
                LEASE_MISMATCH
            );
            Ok(grant)
        }
        Ok(_) => bail!(UNEXPECTED_REPLY),
        Err(error) if is_contended(&error) => Err(rejected(TAKEN_CONTROL_HINT)),
        Err(error) => Err(error.into()),
    }
}

fn is_contended(error: &ClientError) -> bool {
    matches!(
        error,
        ClientError::Remote(PublicError::Conflict | PublicError::Unauthorized)
    )
}

async fn observed_mode(client: &mut LocalClient, target: RuntimeTarget) -> Result<LeaseMode> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    match client
        .call(
            target.address.hub_id,
            Command::ObserveLease { target },
            deadline,
        )
        .await?
    {
        Reply::LeaseObservation { lease } => Ok(match lease {
            Some(lease) => LeaseMode::TakeControl {
                observed_generation: lease.fields().generation,
            },
            None => LeaseMode::Acquire {},
        }),
        _ => bail!(UNEXPECTED_REPLY),
    }
}

pub fn prompt_request(snapshot: &AgentSnapshot, text: String) -> Result<AgentControlRequest> {
    let prompt = Prompt::new(text).map_err(|_| rejected(PROMPT_INVALID))?;
    build(snapshot, AgentAction::SubmitPrompt { prompt })
}

pub fn interrupt_request(snapshot: &AgentSnapshot) -> Result<AgentControlRequest> {
    build(snapshot, AgentAction::Interrupt {})
}

pub fn respond_request(snapshot: &AgentSnapshot, choice: String) -> Result<AgentControlRequest> {
    let interaction = snapshot
        .fields()
        .interaction
        .as_ref()
        .ok_or_else(|| rejected("the agent has no pending interaction"))?;
    build(
        snapshot,
        AgentAction::Respond {
            interaction: interaction.reference(),
            response: InteractionResponse::Choice { choice },
        },
    )
}

fn build(snapshot: &AgentSnapshot, action: AgentAction) -> Result<AgentControlRequest> {
    let request = AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        target: snapshot.cursor().target(),
        expected_cursor: snapshot.cursor(),
        action: action.clone(),
    })?;
    request
        .validate_against(snapshot)
        .map_err(|error| rejected(readiness_message(snapshot, action, &error)))?;
    Ok(request)
}

pub async fn execute(
    client: &mut LocalClient,
    request: AgentControlRequest,
    grant: &LeaseGrant,
) -> Result<Execution> {
    let destination = request.fields().target.address.hub_id;
    let deadline = Instant::now() + CONTROL_TIMEOUT;
    match client
        .call(
            destination,
            Command::AgentControl {
                request,
                grant: grant.clone(),
            },
            deadline,
        )
        .await
    {
        Ok(Reply::AgentControl { receipt }) => Ok(Execution::Receipt(receipt)),
        Ok(_) => bail!(UNEXPECTED_REPLY),
        Err(error) if error.outcome_is_unknown() => Ok(Execution::Unknown),
        Err(error) => Err(error.into()),
    }
}

pub async fn release(client: &mut LocalClient, grant: &LeaseGrant) {
    if !client.is_usable() {
        return;
    }
    let deadline = Instant::now() + RELEASE_TIMEOUT;
    let _ = client
        .call(
            grant.fields().target.address.hub_id,
            Command::Release {
                grant: grant.clone(),
            },
            deadline,
        )
        .await;
}
