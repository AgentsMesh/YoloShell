use crate::{ClientError, exchange::exchange};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply, Request};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::{Framed, TransportError};

/// One sequential Connector role channel. Cancellation retires it; requests are never replayed.
pub struct Client<S> {
    hub: HubId,
    channel: Option<Framed<S>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    pub async fn open(stream: S, expected: HubId, deadline: Instant) -> Result<Self, ClientError> {
        let mut client = Self {
            hub: expected,
            channel: Some(Framed::new(stream)),
        };
        let request = Request::new(expected, Command::Hello {}).map_err(ClientError::Validation)?;
        let reply = client.call(request, deadline).await?;
        if !matches!(reply, Reply::Hello { hub } if hub == expected) {
            return Err(ClientError::InvalidResponse(ConnectorError::InvalidRequest));
        }
        Ok(client)
    }

    pub fn hub_id(&self) -> HubId {
        self.hub
    }
    pub fn is_usable(&self) -> bool {
        self.channel.as_ref().is_some_and(Framed::is_usable)
    }

    pub async fn call(
        &mut self,
        request: Request,
        deadline: Instant,
    ) -> Result<Reply, ClientError> {
        request.validate().map_err(ClientError::Validation)?;
        if request.hub != self.hub {
            return Err(ClientError::Validation(ConnectorError::Unauthorized));
        }
        if Instant::now() >= deadline {
            return Err(ClientError::Transport(TransportError::Deadline));
        }
        let mut channel = self.channel.take().ok_or(ClientError::Poisoned)?;
        let result = exchange(&mut channel, &request, deadline).await;
        if result
            .as_ref()
            .map_or_else(|failure| failure.reusable, |_| true)
        {
            self.channel = Some(channel);
        }
        result.map_err(|failure| failure.error)
    }

    pub async fn close(&mut self, deadline: Instant) -> Result<(), ClientError> {
        self.channel
            .take()
            .ok_or(ClientError::Poisoned)?
            .close(deadline)
            .await?;
        Ok(())
    }
}
