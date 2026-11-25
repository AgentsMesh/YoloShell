use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Instant;
use yolo_hub_client::{Client, ClientError};
use yolo_hub_jsonrpc_transport::{Framed, Incoming, TransportError};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, OutputCursor, PublicError, TerminalEvent,
};

/// A dedicated, read-only terminal stream. Writer commands use another client.
/// Cancellation retires the stream; reconnect with the last accepted cursor.
pub struct Attachment<S> {
    initial: Option<AttachResult>,
    cursor: OutputCursor,
    channel: Option<Framed<S>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Attachment<S> {
    pub async fn open(
        mut client: Client<S>,
        request: AttachRequest,
        deadline: Instant,
    ) -> Result<Self, ClientError> {
        let reply = client
            .call(
                request.target().address.hub_id,
                Command::Attach { request },
                deadline,
            )
            .await?;
        let Reply::Attached {
            attachment: initial,
        } = reply
        else {
            return Err(ClientError::InvalidResponse(PublicError::InvalidPayload));
        };
        let cursor = initial
            .validate_for_request(request)
            .map_err(ClientError::InvalidResponse)?;
        Ok(Self {
            initial: Some(initial),
            cursor,
            channel: Some(client.into_channel()?),
        })
    }

    pub fn initial(&self) -> &AttachResult {
        self.initial
            .as_ref()
            .expect("attachment seed was already consumed")
    }

    /// Hands the one-shot seed to the caller; later `initial` calls panic.
    pub fn take_initial(&mut self) -> Option<AttachResult> {
        self.initial.take()
    }
    pub fn cursor(&self) -> OutputCursor {
        self.cursor
    }
    pub fn is_usable(&self) -> bool {
        self.channel.as_ref().is_some_and(Framed::is_usable)
    }

    pub async fn next_event(&mut self, deadline: Instant) -> Result<TerminalEvent, ClientError> {
        if Instant::now() >= deadline {
            return Err(ClientError::Transport(TransportError::Deadline));
        }
        let mut channel = self.channel.take().ok_or(ClientError::Poisoned)?;
        let event = match yolo_hub_terminal_wire::receive_event(&mut channel, deadline).await? {
            Incoming::Message(event) => event,
            Incoming::Eof => return Err(ClientError::Transport(TransportError::TruncatedFrame)),
        };
        self.cursor = validate_event(&event, self.cursor).map_err(ClientError::InvalidResponse)?;
        if !event.is_terminal() {
            self.channel = Some(channel);
        }
        Ok(event)
    }

    pub async fn detach(mut self, deadline: Instant) -> Result<(), ClientError> {
        self.channel
            .take()
            .ok_or(ClientError::Poisoned)?
            .close(deadline)
            .await?;
        Ok(())
    }
}

pub fn validate_event(
    event: &TerminalEvent,
    cursor: OutputCursor,
) -> Result<OutputCursor, PublicError> {
    match event.advance(cursor) {
        Err(PublicError::OutputGap) if matches!(event, TerminalEvent::ResyncRequired { .. }) => {
            Ok(cursor)
        }
        Err(PublicError::Unreachable) if matches!(event, TerminalEvent::Unreachable { .. }) => {
            Ok(cursor)
        }
        outcome => outcome,
    }
}
