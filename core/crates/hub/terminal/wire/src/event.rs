use crate::{Error, metadata};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_jsonrpc_protocol::{JSONRPC_VERSION, TERMINAL_EVENT_METHOD};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_terminal_protocol::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Notification {
    jsonrpc: String,
    method: String,
    params: Event,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Output {
        chunk: metadata::Chunk,
    },
    Heartbeat {
        cursor: OutputCursor,
    },
    Exited {
        cursor: OutputCursor,
        code: Option<i32>,
        signal: Option<i32>,
    },
    ResyncRequired {
        target: RuntimeTarget,
    },
    Unreachable {
        target: RuntimeTarget,
    },
}

pub async fn send_event<S: AsyncWrite + Unpin>(
    channel: &mut Framed<S>,
    event: &TerminalEvent,
    deadline: Instant,
) -> Result<(), Error> {
    event.validate()?;
    let params = match event {
        TerminalEvent::Output { chunk } => Event::Output {
            chunk: chunk.into(),
        },
        TerminalEvent::Heartbeat { cursor } => Event::Heartbeat { cursor: *cursor },
        TerminalEvent::Exited {
            cursor,
            code,
            signal,
        } => Event::Exited {
            cursor: *cursor,
            code: *code,
            signal: *signal,
        },
        TerminalEvent::ResyncRequired { target } => Event::ResyncRequired { target: *target },
        TerminalEvent::Unreachable { target } => Event::Unreachable { target: *target },
    };
    channel
        .send(
            &Notification {
                jsonrpc: JSONRPC_VERSION.into(),
                method: TERMINAL_EVENT_METHOD.into(),
                params,
            },
            deadline,
        )
        .await?;
    if let TerminalEvent::Output { chunk } = event {
        channel
            .send_binary(&chunk.fields().data, MAX_OUTPUT_CHUNK_BYTES, deadline)
            .await?;
    }
    Ok(())
}

pub async fn receive_event<S: AsyncRead + Unpin>(
    channel: &mut Framed<S>,
    deadline: Instant,
) -> Result<Incoming<TerminalEvent>, Error> {
    let result = receive(channel, deadline).await;
    if result.is_err() {
        channel.retire();
    }
    result
}

async fn receive<S: AsyncRead + Unpin>(
    channel: &mut Framed<S>,
    deadline: Instant,
) -> Result<Incoming<TerminalEvent>, Error> {
    let Incoming::Message(notification) = channel.receive::<Notification>(deadline).await? else {
        return Ok(Incoming::Eof);
    };
    if notification.jsonrpc != JSONRPC_VERSION || notification.method != TERMINAL_EVENT_METHOD {
        return Err(PublicError::InvalidPayload.into());
    }
    let event = match notification.params {
        Event::Output { chunk } => {
            let data = channel
                .receive_binary(chunk.byte_length, MAX_OUTPUT_CHUNK_BYTES, deadline)
                .await?;
            TerminalEvent::Output {
                chunk: OutputChunk::new(OutputChunkFields {
                    target: chunk.target,
                    sequence: chunk.sequence,
                    data,
                })?,
            }
        }
        Event::Heartbeat { cursor } => TerminalEvent::Heartbeat { cursor },
        Event::Exited {
            cursor,
            code,
            signal,
        } => TerminalEvent::Exited {
            cursor,
            code,
            signal,
        },
        Event::ResyncRequired { target } => TerminalEvent::ResyncRequired { target },
        Event::Unreachable { target } => TerminalEvent::Unreachable { target },
    };
    event.validate()?;
    Ok(Incoming::Message(event))
}
