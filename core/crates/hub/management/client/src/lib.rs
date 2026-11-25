mod error;
pub use error::ManagementError;
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::{Command, Reply};

/// Owns one management connection and one absolute deadline. Never retries a mutation.
pub async fn execute(
    socket: &Path,
    entry: HubId,
    command: Command,
    deadline: Instant,
) -> Result<ConfigurationSnapshot, ManagementError> {
    let mutation = command.mutates();
    match execute_reply(socket, entry, command, deadline).await? {
        Reply::Configuration { snapshot } => Ok(snapshot),
        _ if mutation => Err(ManagementError::OutcomeUnknown),
        _ => Err(ManagementError::Unavailable),
    }
}

/// Executes a local-only management request through the selected Hub entry and
/// returns the typed reply. Owns one connection and one absolute deadline.
pub async fn execute_reply(
    socket: &Path,
    entry: HubId,
    command: Command,
    deadline: Instant,
) -> Result<Reply, ManagementError> {
    execute_endpoint_reply(&HubEndpoint::local(socket), entry, command, deadline).await
}

/// Executes a local-only management request through an explicit endpoint.
pub async fn execute_endpoint_reply(
    endpoint: &HubEndpoint,
    entry: HubId,
    command: Command,
    deadline: Instant,
) -> Result<Reply, ManagementError> {
    if !command.local_only() {
        return Err(ManagementError::InvalidRequest);
    }
    command
        .validate_origin(entry)
        .map_err(|_| ManagementError::InvalidRequest)?;
    let mut client = endpoint.connect(entry, deadline).await?;
    client
        .call(entry, command, deadline)
        .await
        .map_err(ManagementError::from)
}

/// Executes a local-only management request through the selected Hub entry.
/// Owns one connection and one absolute deadline. Never retries a mutation.
pub async fn execute_endpoint(
    endpoint: &HubEndpoint,
    entry: HubId,
    command: Command,
    deadline: Instant,
) -> Result<ConfigurationSnapshot, ManagementError> {
    let mutation = command.mutates();
    match execute_endpoint_reply(endpoint, entry, command, deadline).await? {
        Reply::Configuration { snapshot } => Ok(snapshot),
        _ if mutation => Err(ManagementError::OutcomeUnknown),
        _ => Err(ManagementError::Unavailable),
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
