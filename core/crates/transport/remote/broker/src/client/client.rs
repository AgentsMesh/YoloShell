use super::client_io::{
    ConnectedBroker, bootstrap_on, request_on, retire_on, shutdown_on, status_on,
};
use super::protocol::*;
use super::socket_identity::SocketIdentity;
use super::timing::BROKER_CLIENT_OPERATION_TIMEOUT;
use crate::protocol::ProtocolError;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrokerClientError {
    #[error(transparent)]
    HubProtocol(#[from] yolo_broker_hub_protocol::BrokerHubError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("broker socket is unsafe: {0}")]
    UnsafeSocket(String),
    #[error("broker protocol version is incompatible")]
    IncompatibleProtocol,
    #[error("broker response request id differs")]
    RequestMismatch,
    #[error("{code}: {message}")]
    Remote { code: String, message: String },
    #[error("broker returned an unexpected result")]
    UnexpectedResult,
    #[error("broker client deadline exceeded")]
    Deadline,
    #[error("{0}")]
    Retirement(String),
}

#[derive(Clone, Debug)]
pub struct BrokerClient {
    socket: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrokerTransition {
    Current(BrokerStatus),
    Pending(BrokerStatus),
    Shutdown,
}

impl BrokerClient {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    pub async fn status(&self) -> Result<BrokerStatus, BrokerClientError> {
        let mut connection = self.connect(None, operation_deadline()).await?;
        let deadline = connection.deadline();
        let status = status_on(connection.stream(), deadline).await?;
        connection.verify_path()?;
        Ok(status)
    }

    pub async fn run_command(
        &self,
        request: BrokerCommandRequest,
    ) -> Result<BrokerResult, BrokerClientError> {
        let result = self.request(BrokerRequest::Command(request)).await?;
        matches!(result, BrokerResult::Command { .. })
            .then_some(result)
            .ok_or(BrokerClientError::UnexpectedResult)
    }

    pub async fn working_directory(&self) -> Result<BrokerResult, BrokerClientError> {
        let result = self.request(BrokerRequest::WorkingDirectory).await?;
        matches!(result, BrokerResult::WorkingDirectory { .. })
            .then_some(result)
            .ok_or(BrokerClientError::UnexpectedResult)
    }

    pub async fn read_file(
        &self,
        request: BrokerFileRequest,
    ) -> Result<BrokerResult, BrokerClientError> {
        let result = self.request(BrokerRequest::FileRead(request)).await?;
        matches!(result, BrokerResult::FileRead { .. })
            .then_some(result)
            .ok_or(BrokerClientError::UnexpectedResult)
    }

    pub async fn shutdown(&self, expected_revision: String) -> Result<(), BrokerClientError> {
        let mut connection = self.connect(None, operation_deadline()).await?;
        let deadline = connection.deadline();
        let status = status_on(connection.stream(), deadline).await?;
        shutdown_on(
            connection.stream(),
            expected_revision,
            status.instance_id,
            deadline,
        )
        .await
    }

    pub(super) async fn retire_at(
        &self,
        expected_host_id: &str,
        expected: SocketIdentity,
        deadline: tokio::time::Instant,
    ) -> Result<(), BrokerClientError> {
        let mut connection = self.connect(Some(expected), deadline).await?;
        let status = status_on(connection.stream(), deadline).await?;
        super::retirement::validate_retirement_status(&status, expected_host_id)?;
        retire_on(
            connection.stream(),
            status.config_revision,
            status.instance_id,
            deadline,
        )
        .await
    }

    pub(super) async fn transition_at(
        &self,
        desired_revision: &str,
        expected: SocketIdentity,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerTransition, BrokerClientError> {
        let mut connection = self.connect(Some(expected), deadline).await?;
        let status = status_on(connection.stream(), deadline).await?;
        if status.config_revision == desired_revision
            && (status.supports(super::constants::CONNECTION_BOOTSTRAP_CAPABILITY)
                || status.supports(yolo_broker_hub_protocol::HUB_TRANSPORT_CAPABILITY))
            && status.supports(super::constants::WORKING_DIRECTORY_CAPABILITY)
        {
            connection.verify_path()?;
            return Ok(if status.adopted {
                BrokerTransition::Current(status)
            } else {
                BrokerTransition::Pending(status)
            });
        }
        shutdown_on(
            connection.stream(),
            status.config_revision.clone(),
            status.instance_id,
            deadline,
        )
        .await?;
        Ok(BrokerTransition::Shutdown)
    }

    pub async fn request(&self, request: BrokerRequest) -> Result<BrokerResult, BrokerClientError> {
        self.request_inner(request, None).await
    }

    async fn request_inner(
        &self,
        request: BrokerRequest,
        expected: Option<SocketIdentity>,
    ) -> Result<BrokerResult, BrokerClientError> {
        let mut connection = self.connect(expected, operation_deadline()).await?;
        let deadline = connection.deadline();
        request_on(connection.stream(), request, deadline).await
    }

    async fn connect(
        &self,
        expected: Option<SocketIdentity>,
        deadline: tokio::time::Instant,
    ) -> Result<ConnectedBroker, BrokerClientError> {
        ConnectedBroker::connect(self.socket.clone(), expected, deadline).await
    }
}

fn operation_deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + BROKER_CLIENT_OPERATION_TIMEOUT
}

include!("client_bootstrap.rs");
#[path = "client_hub_peer.rs"]
mod hub_peer;
#[path = "client_provisioning.rs"]
mod provisioning;
