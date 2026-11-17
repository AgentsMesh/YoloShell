use super::super::bridge;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use yolo_client::{Client, ClientError, RpcConnection};
use yolo_runtime::host::{ListSessionsRequest, ListSessionsResult};
use yolo_runtime::model::TerminalView;
use yolo_runtime::paths::Paths;
use yolo_runtime::protocol::method;

#[cfg(test)]
const OPERATION_TIMEOUT: Duration = Duration::from_secs(8);
const RETRY_DELAYS: [Duration; 2] = [Duration::from_millis(25), Duration::from_millis(100)];

enum Endpoint {
    Existing,
    Managed { paths: Paths, socket: PathBuf },
}

pub(super) struct CatalogConnection {
    client: Client,
    endpoint: Endpoint,
    connection: Option<RpcConnection>,
}

impl CatalogConnection {
    pub(super) fn existing(client: Client) -> Self {
        Self {
            client,
            endpoint: Endpoint::Existing,
            connection: None,
        }
    }

    pub(super) fn managed(paths: Paths, socket: PathBuf) -> Result<Self> {
        let client = Client::managed_local(paths.home(), &socket)?;
        Ok(Self {
            client,
            endpoint: Endpoint::Managed { paths, socket },
            connection: None,
        })
    }

    #[cfg(test)]
    pub(super) async fn list_sessions(&mut self) -> Result<Vec<TerminalView>> {
        self.list_sessions_with_timeout(OPERATION_TIMEOUT).await
    }

    pub(super) async fn list_sessions_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Vec<TerminalView>> {
        let outcome = tokio::time::timeout(timeout, self.list_with_retries()).await;
        if outcome.is_err() {
            self.connection = None;
        }
        outcome.map_err(anyhow::Error::from)?
    }

    async fn list_with_retries(&mut self) -> Result<Vec<TerminalView>> {
        for delay in RETRY_DELAYS.into_iter().map(Some).chain([None]) {
            match self.list_once().await {
                Ok(sessions) => return Ok(sessions),
                Err(error) if is_transport_error(&error) && delay.is_some() => {
                    self.connection = None;
                    tokio::time::sleep(delay.expect("guarded retry delay")).await;
                }
                Err(error) => {
                    self.connection = None;
                    return Err(error);
                }
            }
        }
        unreachable!("bounded retry iterator always returns")
    }

    async fn list_once(&mut self) -> Result<Vec<TerminalView>> {
        if self.connection.is_none() {
            self.connection = Some(self.connect().await?);
        }
        let connection = self
            .connection
            .as_mut()
            .expect("connection was established");
        let request = connection.request::<_, ListSessionsResult>(
            method::LIST_SESSIONS,
            &ListSessionsRequest {
                include_archived: false,
                require_complete: true,
            },
        );
        let result = request.await?;
        if !result.complete {
            return Err(ClientError::IncompleteCatalogInventory.into());
        }
        Ok(result.sessions)
    }

    async fn connect(&self) -> Result<RpcConnection> {
        if let Endpoint::Managed { paths, socket } = &self.endpoint {
            bridge::ensure_host_running(paths, socket).await?;
        }
        self.client.connect().await.map_err(Into::into)
    }
}

pub(super) fn is_transport_error(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ClientError>(),
        Some(ClientError::Deadline | ClientError::Protocol(_))
    ) || error
        .downcast_ref::<tokio::time::error::Elapsed>()
        .is_some()
}
