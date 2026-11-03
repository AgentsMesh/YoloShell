use crate::{Error, STREAM_ENCODING, metadata};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_jsonrpc_protocol::{Answer, Call, JSONRPC_VERSION};
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultBody {
    stream: String,
    attachment: metadata::Attachment,
}

pub async fn send_attachment<S: AsyncWrite + Unpin>(
    channel: &mut Framed<S>,
    call: &Call,
    initial: &AttachResult,
    deadline: Instant,
) -> Result<(), Error> {
    let Command::Attach { request } = call.fields().command else {
        return Err(PublicError::InvalidRequest.into());
    };
    if initial.fields().target.address.hub_id != call.fields().destination {
        return Err(PublicError::OriginMismatch.into());
    }
    initial.validate_for_request(request)?;
    let body = ResultBody {
        stream: STREAM_ENCODING.into(),
        attachment: initial.into(),
    };
    let answer = Answer {
        jsonrpc: JSONRPC_VERSION.into(),
        id: Some(call.id.clone()),
        result: Some(serde_json::to_value(body)?),
        error: None,
    };
    channel.send(&answer, deadline).await?;
    if let Some(snapshot) = &initial.fields().snapshot {
        channel
            .send_binary(&snapshot.fields().data, MAX_SNAPSHOT_BYTES, deadline)
            .await?;
    }
    for chunk in &initial.fields().chunks {
        channel
            .send_binary(&chunk.fields().data, MAX_OUTPUT_CHUNK_BYTES, deadline)
            .await?;
    }
    if let Some(history) = &initial.fields().history {
        channel
            .send_binary(&history.fields().data, MAX_SNAPSHOT_BYTES, deadline)
            .await?;
    }
    Ok(())
}

pub async fn receive_attachment<S: AsyncRead + Unpin>(
    channel: &mut Framed<S>,
    answer: Answer,
    call: &Call,
    deadline: Instant,
) -> Result<Reply, Error> {
    let result = receive(channel, answer, call, deadline).await;
    if result.is_err() {
        channel.retire();
    }
    result
}

async fn receive<S: AsyncRead + Unpin>(
    channel: &mut Framed<S>,
    answer: Answer,
    call: &Call,
    deadline: Instant,
) -> Result<Reply, Error> {
    if answer.jsonrpc != JSONRPC_VERSION
        || answer.id.as_ref() != Some(&call.id)
        || answer.error.is_some()
    {
        return Err(PublicError::InvalidRequest.into());
    }
    let Command::Attach { request } = call.fields().command else {
        return Err(PublicError::InvalidRequest.into());
    };
    let body: ResultBody =
        serde_json::from_value(answer.result.ok_or(PublicError::InvalidPayload)?)?;
    let initial = body.attachment;
    if body.stream != STREAM_ENCODING
        || initial.target != request.target()
        || initial.chunks.len() > MAX_ATTACH_CHUNKS
    {
        return Err(PublicError::InvalidPayload.into());
    }
    if initial.snapshot.as_ref().is_some_and(|snapshot| {
        snapshot.target != initial.target
            || snapshot.byte_length > MAX_SNAPSHOT_BYTES
            || !initial.chunks.is_empty()
            || !initial.resynchronized
    }) || initial.history.as_ref().is_some_and(|history| {
        history.target != initial.target
            || history.byte_length > MAX_SNAPSHOT_BYTES
            || initial.snapshot.is_some()
            || !initial.chunks.is_empty()
            || initial.resynchronized
    }) || initial.chunks.iter().any(|chunk| {
        chunk.target != initial.target
            || chunk.byte_length == 0
            || chunk.byte_length > MAX_OUTPUT_CHUNK_BYTES
    }) {
        return Err(PublicError::InvalidPayload.into());
    }
    let bytes = initial
        .chunks
        .iter()
        .try_fold(0usize, |total, chunk| total.checked_add(chunk.byte_length));
    if bytes.is_none_or(|bytes| bytes > MAX_ATTACH_OUTPUT_BYTES) {
        return Err(PublicError::InvalidPayload.into());
    }
    let snapshot = if let Some(meta) = initial.snapshot {
        let data = channel
            .receive_binary(meta.byte_length, MAX_SNAPSHOT_BYTES, deadline)
            .await?;
        Some(TerminalSnapshot::new(TerminalSnapshotFields {
            target: meta.target,
            barrier_sequence: meta.barrier_sequence,
            size: meta.size,
            data,
        })?)
    } else {
        None
    };
    let mut chunks = Vec::with_capacity(initial.chunks.len());
    for meta in initial.chunks {
        let data = channel
            .receive_binary(meta.byte_length, MAX_OUTPUT_CHUNK_BYTES, deadline)
            .await?;
        chunks.push(OutputChunk::new(OutputChunkFields {
            target: meta.target,
            sequence: meta.sequence,
            data,
        })?);
    }
    let history = if let Some(meta) = initial.history {
        let data = channel
            .receive_binary(meta.byte_length, MAX_SNAPSHOT_BYTES, deadline)
            .await?;
        Some(TerminalHistory::new(TerminalHistoryFields {
            target: meta.target,
            barrier_sequence: meta.barrier_sequence,
            size: meta.size,
            data,
        })?)
    } else {
        None
    };
    let attachment = AttachResult::new(AttachResultFields {
        target: initial.target,
        snapshot,
        chunks,
        history,
        next_sequence: initial.next_sequence,
        resynchronized: initial.resynchronized,
    })?;
    attachment.validate_for_request(request)?;
    Ok(Reply::Attached { attachment })
}
