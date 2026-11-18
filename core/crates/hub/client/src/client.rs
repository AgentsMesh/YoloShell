use crate::ClientError;
use crate::exchange::exchange;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::{Framed, TransportError};
use yolo_hub_rpc_protocol::{Command, Hello, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

/// Each independently authenticated control/attachment channel owns one client.
/// No request is automatically retried, including after a reconnect.
pub struct Client<S> {
    channel: Option<Framed<S>>,
    hello: Hello,
    phase: Phase,
}

#[derive(PartialEq)]
enum Phase {
    Control,
    Terminal,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    /// The caller supplies an authenticated transport and its pinned Hub identity.
    pub async fn open(stream: S, expected: HubId, deadline: Instant) -> Result<Self, ClientError> {
        let mut channel = Framed::new(stream);
        let request = Request::new(expected, Command::Hello {}).map_err(ClientError::Validation)?;
        let reply = exchange(&mut channel, &request, deadline)
            .await
            .map_err(|failure| failure.error)?;
        let Reply::Hello { hello } = reply else {
            return Err(ClientError::InvalidResponse(PublicError::InvalidPayload));
        };
        Ok(Self {
            channel: Some(channel),
            hello,
            phase: Phase::Control,
        })
    }

    pub fn hello(&self) -> &Hello {
        &self.hello
    }
    pub fn is_usable(&self) -> bool {
        self.phase == Phase::Control && self.channel.as_ref().is_some_and(Framed::is_usable)
    }

    pub fn into_channel(self) -> Result<Framed<S>, ClientError> {
        self.channel.ok_or(ClientError::Poisoned)
    }

    pub async fn call(
        &mut self,
        destination: HubId,
        command: Command,
        deadline: Instant,
    ) -> Result<Reply, ClientError> {
        if self.phase != Phase::Control {
            return Err(ClientError::TerminalStream);
        }
        if !self.hello.methods.contains(&command.method()) {
            return Err(ClientError::Unsupported);
        }
        let request = Request::new(destination, command).map_err(ClientError::Validation)?;
        // Preflight failures cannot have delivered a side effect and keep the channel.
        if Instant::now() >= deadline {
            return Err(ClientError::Transport(TransportError::Deadline));
        }
        let mut channel = self.channel.take().ok_or(ClientError::Poisoned)?;
        let reply = exchange(&mut channel, &request, deadline).await;
        if matches!(reply, Ok(Reply::Attached { .. })) {
            self.phase = Phase::Terminal;
        }
        if reply
            .as_ref()
            .map_or_else(|failure| failure.reusable, |_| true)
        {
            self.channel = Some(channel);
        }
        reply.map_err(|failure| failure.error)
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
