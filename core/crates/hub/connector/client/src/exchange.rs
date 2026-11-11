use crate::ClientError;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_connector_jsonrpc::{Answer, Call};
use yolo_hub_connector_protocol::{Reply, Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming, TransportError};

pub(crate) struct Failure {
    pub error: ClientError,
    pub reusable: bool,
}

pub(crate) async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    channel: &mut Framed<S>,
    request: &Request,
    deadline: Instant,
) -> Result<Reply, Failure> {
    let mutation = request.command.mutates();
    let call = Call::from_request(request.clone());
    channel.send(&call, deadline).await.map_err(|error| {
        let reusable = channel.is_usable();
        Failure {
            error: classify(error, mutation && !reusable),
            reusable,
        }
    })?;
    let response: Answer = match channel.receive(deadline).await.map_err(|error| Failure {
        error: classify(error, mutation),
        reusable: false,
    })? {
        Incoming::Message(response) => response,
        Incoming::Eof => {
            return Err(Failure {
                error: classify(TransportError::TruncatedFrame, mutation),
                reusable: false,
            });
        }
    };
    let reply = response.reply(&call).map_err(|error| Failure {
        error: if mutation {
            ClientError::OutcomeUnknownResponse(error)
        } else {
            ClientError::InvalidResponse(error)
        },
        reusable: false,
    })?;
    match reply {
        Reply::Error { code } => Err(Failure {
            error: ClientError::Remote(code),
            reusable: true,
        }),
        reply => Ok(reply),
    }
}

fn classify(error: TransportError, mutation: bool) -> ClientError {
    if mutation {
        ClientError::OutcomeUnknown(error)
    } else {
        ClientError::Transport(error)
    }
}
