use super::deadline::ClientDeadline;
use super::*;
use crate::local_unix::LocalUnixStream as UnixStream;
use crate::protocol::{RequestEnvelope, to_value, validate_response};

#[path = "connection/authentication.rs"]
mod authentication;

pub struct RpcConnection {
    peer_credentials: tokio::net::unix::UCred,
    stream: Option<yolo_runtime_rpc::RuntimeConnection>,
    authenticated_status: Option<HostStatus>,
    operation_timeout: std::time::Duration,
}

impl Client {
    pub async fn connect(&self) -> Result<RpcConnection, ClientError> {
        self.connect_until(self.operation_deadline()).await
    }

    pub(super) async fn connect_until(
        &self,
        deadline: ClientDeadline,
    ) -> Result<RpcConnection, ClientError> {
        let connected = deadline.run(UnixStream::connect(&self.socket)).await?;
        let connected = connected.map_err(ProtocolError::Io)?;
        local_transport::verify_peer_uid(&connected)?;
        let mut connection = RpcConnection {
            peer_credentials: connected.peer_cred().map_err(ProtocolError::Io)?,
            stream: Some(
                yolo_runtime_rpc::RuntimeConnection::open(connected, deadline.instant()).await?,
            ),
            authenticated_status: None,
            operation_timeout: self.operation_timeout,
        };
        if let Some(expected_home) = &self.expected_home {
            connection
                .authenticate_local(expected_home, &self.socket, deadline)
                .await?;
        }
        Ok(connection)
    }

    pub async fn raw<T: Serialize>(
        &self,
        method_name: &str,
        payload: &T,
    ) -> Result<ResponseEnvelope, ClientError> {
        let timeout = constants::request_operation_timeout(self.operation_timeout, method_name);
        let deadline = ClientDeadline::after(timeout);
        self.connect_until(deadline)
            .await?
            .raw_until(method_name, payload, deadline)
            .await
    }

    pub async fn request<T: Serialize, R: DeserializeOwned>(
        &self,
        method_name: &str,
        payload: &T,
    ) -> Result<R, ClientError> {
        let timeout = constants::request_operation_timeout(self.operation_timeout, method_name);
        self.request_until(method_name, payload, ClientDeadline::after(timeout))
            .await
    }

    pub(super) async fn request_until<T: Serialize, R: DeserializeOwned>(
        &self,
        method_name: &str,
        payload: &T,
        deadline: ClientDeadline,
    ) -> Result<R, ClientError> {
        self.connect_until(deadline)
            .await?
            .request_until(method_name, payload, deadline)
            .await
    }
}

impl RpcConnection {
    pub fn peer_credentials(&self) -> &tokio::net::unix::UCred {
        &self.peer_credentials
    }
    pub async fn forward(
        &mut self,
        request: &RequestEnvelope,
    ) -> Result<yolo_capnp_rpc::RemoteReply, ClientError> {
        let timeout = constants::request_operation_timeout(self.operation_timeout, &request.method);
        let deadline = ClientDeadline::after(timeout);
        let Some(mut stream) = self.stream.take() else {
            return Err(ProtocolError::ConnectionPoisoned.into());
        };
        let reply = stream.forward(request, deadline.instant()).await?;
        self.stream = Some(stream);
        Ok(reply)
    }
    pub(super) fn take_authenticated_status(&mut self) -> Option<HostStatus> {
        self.authenticated_status.take()
    }

    pub fn into_channel(mut self) -> Result<yolo_runtime_rpc::RuntimeConnection, ClientError> {
        self.stream
            .take()
            .ok_or_else(|| ProtocolError::ConnectionPoisoned.into())
    }

    pub async fn raw<T: Serialize>(
        &mut self,
        method_name: &str,
        payload: &T,
    ) -> Result<ResponseEnvelope, ClientError> {
        let timeout = constants::request_operation_timeout(self.operation_timeout, method_name);
        let deadline = ClientDeadline::after(timeout);
        self.raw_until(method_name, payload, deadline).await
    }

    pub(super) async fn raw_until<T: Serialize>(
        &mut self,
        method_name: &str,
        payload: &T,
        deadline: ClientDeadline,
    ) -> Result<ResponseEnvelope, ClientError> {
        let request =
            RequestEnvelope::new(Uuid::new_v4().to_string(), method_name, to_value(payload)?);
        let Some(mut stream) = self.stream.take() else {
            return Err(ProtocolError::ConnectionPoisoned.into());
        };
        let response = stream.exchange(&request, deadline.instant()).await?;
        if let Err(error) = validate_response(&request, &response) {
            return Err(ClientError::Protocol(error));
        }
        self.stream = Some(stream);
        Ok(response)
    }

    pub async fn request<T: Serialize, R: DeserializeOwned>(
        &mut self,
        method_name: &str,
        payload: &T,
    ) -> Result<R, ClientError> {
        let timeout = constants::request_operation_timeout(self.operation_timeout, method_name);
        let deadline = ClientDeadline::after(timeout);
        self.request_until(method_name, payload, deadline).await
    }

    pub async fn input_with_id(
        &mut self,
        request: &InputRequest,
    ) -> Result<InputResult, ClientError> {
        self.request(crate::protocol::method::TERMINAL_INPUT, request)
            .await
    }

    pub(super) async fn request_until<T: Serialize, R: DeserializeOwned>(
        &mut self,
        method_name: &str,
        payload: &T,
        deadline: ClientDeadline,
    ) -> Result<R, ClientError> {
        let response = self.raw_until(method_name, payload, deadline).await?;
        match decode_response(response) {
            Err(error @ ClientError::Protocol(_)) => Err(self.poison(error)),
            result => result,
        }
    }

    fn poison(&mut self, error: ClientError) -> ClientError {
        self.stream = None;
        error
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/connection_deadline_tests.rs"]
mod deadline_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/connection_response_tests.rs"]
mod response_tests;
