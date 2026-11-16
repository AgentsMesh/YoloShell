use crate::ClientError;
use serde::Serialize;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming, TransportError};
use yolo_hub_rpc_protocol::{Reply, Request};

pub(crate) struct ExchangeFailure {
    pub error: ClientError,
    pub reusable: bool,
}

pub(crate) async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    channel: &mut Framed<S>,
    request: &Request,
    deadline: Instant,
) -> Result<Reply, ExchangeFailure> {
    let mutation = request.fields().command.mutates();
    let call = Call::from_request(request.clone());
    send(channel, &call, mutation, deadline).await?;
    let response =
        match channel
            .receive::<Answer>(deadline)
            .await
            .map_err(|error| ExchangeFailure {
                error: classify(error, mutation),
                reusable: false,
            })? {
            Incoming::Message(response) => response,
            Incoming::Eof => {
                return Err(ExchangeFailure {
                    error: classify(TransportError::TruncatedFrame, mutation),
                    reusable: false,
                });
            }
        };
    let result = if matches!(
        call.fields().command,
        yolo_hub_rpc_protocol::Command::Attach { .. }
    ) && response.result.is_some()
    {
        yolo_hub_terminal_wire::receive_attachment(channel, response, &call, deadline)
            .await
            .map_err(ClientError::from)
    } else {
        response.reply(&call).map_err(ClientError::InvalidResponse)
    };
    let reply = result.map_err(|error| ExchangeFailure {
        error: if mutation {
            match error {
                ClientError::InvalidResponse(error) => ClientError::OutcomeUnknownResponse(error),
                error => error,
            }
        } else {
            error
        },
        reusable: false,
    })?;
    if let Reply::Error { code } = reply {
        return Err(ExchangeFailure {
            error: ClientError::Remote(code),
            reusable: true,
        });
    }
    Ok(reply)
}

pub(crate) async fn send<S: AsyncWrite + Unpin, T: Serialize>(
    channel: &mut Framed<S>,
    request: &T,
    mutation: bool,
    deadline: Instant,
) -> Result<(), ExchangeFailure> {
    channel.send(request, deadline).await.map_err(|error| {
        // Framed takes its stream only after encoding and deadline preflight.
        let reusable = channel.is_usable();
        ExchangeFailure {
            error: classify(error, mutation && !reusable),
            reusable,
        }
    })
}

fn classify(error: TransportError, mutation: bool) -> ClientError {
    if mutation {
        ClientError::OutcomeUnknown(error)
    } else {
        ClientError::Transport(error)
    }
}
