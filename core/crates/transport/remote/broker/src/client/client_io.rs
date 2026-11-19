use super::client::BrokerClientError;
use super::constants::{BROKER_PROTOCOL_VERSION, BROKER_SOCKET_CHANGED_CONTEXT};
use super::protocol::*;
use super::socket_identity::{SocketIdentity, socket_identity};
use super::sockets::validate_broker_socket;
use crate::local_unix::LocalUnixStream as UnixStream;
use crate::protocol::{ProtocolError, read_frame, write_frame};
use std::{future::Future, path::PathBuf};
use uuid::Uuid;

pub(super) struct ConnectedBroker {
    stream: UnixStream,
    path: PathBuf,
    identity: SocketIdentity,
    deadline: tokio::time::Instant,
}

impl ConnectedBroker {
    pub(super) fn into_stream(self) -> Result<UnixStream, BrokerClientError> {
        self.verify_path()?;
        Ok(self.stream)
    }
    pub(super) async fn connect(
        path: PathBuf,
        expected: Option<SocketIdentity>,
        deadline: tokio::time::Instant,
    ) -> Result<Self, BrokerClientError> {
        require_before(deadline)?;
        validate_broker_socket(&path).map_err(unsafe_socket)?;
        require_before(deadline)?;
        let identity = inspect_before(&path, deadline)?;
        verify_identity(expected.unwrap_or(identity), identity)?;
        let stream = before(deadline, UnixStream::connect(&path))
            .await?
            .map_err(ProtocolError::Io)?;
        verify_identity(identity, inspect_before(&path, deadline)?)?;
        require_before(deadline)?;
        super::sockets::verify_peer(&stream).map_err(unsafe_socket)?;
        require_before(deadline)?;
        Ok(Self {
            stream,
            path,
            identity,
            deadline,
        })
    }

    pub(super) fn stream(&mut self) -> &mut UnixStream {
        &mut self.stream
    }

    pub(super) fn deadline(&self) -> tokio::time::Instant {
        self.deadline
    }

    pub(super) fn verify_path(&self) -> Result<(), BrokerClientError> {
        let actual = inspect_before(&self.path, self.deadline)?;
        verify_identity(self.identity, actual)?;
        require_before(self.deadline)
    }
}

pub(super) async fn status_on(
    stream: &mut UnixStream,
    deadline: tokio::time::Instant,
) -> Result<BrokerStatus, BrokerClientError> {
    match request_on(stream, BrokerRequest::Status, deadline).await? {
        BrokerResult::Status(value) => Ok(value),
        _ => Err(BrokerClientError::UnexpectedResult),
    }
}

pub(super) async fn bootstrap_on(
    stream: &mut UnixStream,
    deadline: tokio::time::Instant,
) -> Result<BrokerStatus, BrokerClientError> {
    match request_on(stream, BrokerRequest::Bootstrap, deadline).await? {
        BrokerResult::Bootstrap(value) => Ok(value),
        _ => Err(BrokerClientError::UnexpectedResult),
    }
}

pub(super) async fn shutdown_on(
    stream: &mut UnixStream,
    expected_revision: String,
    expected_instance_id: Uuid,
    deadline: tokio::time::Instant,
) -> Result<(), BrokerClientError> {
    let requested = expected_revision.clone();
    let request = BrokerRequest::Shutdown {
        expected_revision,
        expected_instance_id,
    };
    match request_on(stream, request, deadline).await? {
        BrokerResult::Shutdown {
            config_revision,
            instance_id,
        } if config_revision == requested && instance_id == expected_instance_id => Ok(()),
        _ => Err(BrokerClientError::UnexpectedResult),
    }
}

pub(super) async fn retire_on(
    stream: &mut UnixStream,
    expected_revision: String,
    expected_instance_id: Uuid,
    deadline: tokio::time::Instant,
) -> Result<(), BrokerClientError> {
    let requested = expected_revision.clone();
    let request = BrokerRequest::Retire {
        expected_revision,
        expected_instance_id,
    };
    match request_on(stream, request, deadline).await? {
        BrokerResult::Retire {
            config_revision,
            instance_id,
        } if config_revision == requested && instance_id == expected_instance_id => Ok(()),
        _ => Err(BrokerClientError::UnexpectedResult),
    }
}

pub(super) async fn request_on(
    stream: &mut UnixStream,
    request: BrokerRequest,
    deadline: tokio::time::Instant,
) -> Result<BrokerResult, BrokerClientError> {
    let request = BrokerRequestEnvelope::new(request);
    before(deadline, write_frame(stream, &request)).await??;
    let response: BrokerResponse = before(deadline, read_frame(stream)).await??;
    decode_response(request.request_id, response)
}

async fn before<F, T>(deadline: tokio::time::Instant, future: F) -> Result<T, BrokerClientError>
where
    F: Future<Output = T>,
{
    require_before(deadline)?;
    let result = tokio::time::timeout_at(deadline, future)
        .await
        .map_err(|_| BrokerClientError::Deadline)?;
    require_before(deadline)?;
    Ok(result)
}

fn require_before(deadline: tokio::time::Instant) -> Result<(), BrokerClientError> {
    (tokio::time::Instant::now() < deadline)
        .then_some(())
        .ok_or(BrokerClientError::Deadline)
}

fn inspect(path: &std::path::Path) -> Result<SocketIdentity, BrokerClientError> {
    socket_identity(path).map_err(|error| unsafe_socket(error.into()))
}

fn inspect_before(
    path: &std::path::Path,
    deadline: tokio::time::Instant,
) -> Result<SocketIdentity, BrokerClientError> {
    require_before(deadline)?;
    let identity = inspect(path)?;
    require_before(deadline)?;
    Ok(identity)
}

fn verify_identity(
    expected: SocketIdentity,
    actual: SocketIdentity,
) -> Result<(), BrokerClientError> {
    if actual != expected {
        return Err(BrokerClientError::UnsafeSocket(
            BROKER_SOCKET_CHANGED_CONTEXT.into(),
        ));
    }
    Ok(())
}

fn unsafe_socket(error: anyhow::Error) -> BrokerClientError {
    BrokerClientError::UnsafeSocket(error.to_string())
}

fn decode_response(
    request_id: Uuid,
    response: BrokerResponse,
) -> Result<BrokerResult, BrokerClientError> {
    if response.protocol_version != BROKER_PROTOCOL_VERSION {
        return Err(BrokerClientError::IncompatibleProtocol);
    }
    if response.request_id != request_id {
        return Err(BrokerClientError::RequestMismatch);
    }
    if let Some(result) = response.result {
        return Ok(result);
    }
    Err(BrokerClientError::Remote {
        code: response.error_code.unwrap_or_default(),
        message: response.error_message.unwrap_or_default(),
    })
}
