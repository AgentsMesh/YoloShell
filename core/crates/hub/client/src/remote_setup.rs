use crate::{Client, ClientError};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use uuid::Uuid;
use yolo_hub_remote_setup_protocol::RemoteSetupHistory;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    /// Reads the connected entry Hub's durable setup checkpoint by operation ID.
    /// This does not start or resume installation and never forwards credentials.
    pub async fn inspect_remote_setup(
        &mut self,
        operation_id: Uuid,
        deadline: Instant,
    ) -> Result<RemoteSetupHistory, ClientError> {
        let entry = self.hello().hub_id;
        match self
            .call(
                entry,
                Command::InspectRemoteSetup { operation_id },
                deadline,
            )
            .await?
        {
            Reply::RemoteSetupHistory { history } => Ok(history),
            _ => Err(ClientError::InvalidResponse(PublicError::InvalidPayload)),
        }
    }
}
