use crate::hub::{
    support::deadline,
    tui_creation_recovery::{audit::Failure, constants::*},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use yolo_hub_jsonrpc_protocol::{Answer, Call, decode};
use yolo_hub_jsonrpc_transport::{FRAME_DELIMITER, MAX_FRAME_BYTES, encode};

pub async fn request(stream: &mut UnixStream) -> Result<Option<Call>, Failure> {
    // Idle connections have no artificial proxy deadline. Once bytes arrive,
    // frame completion is bounded, and dropping the owner closes the socket.
    let first = match stream.read_u8().await {
        Ok(first) => first,
        Err(_) => return Ok(None),
    };
    tokio::time::timeout_at(deadline(), async {
        let mut payload = Vec::new();
        let mut next = first;
        while next != FRAME_DELIMITER {
            if payload.len() >= MAX_FRAME_BYTES {
                return Err(Failure::ReadRequest);
            }
            payload.push(next);
            next = match stream.read_u8().await {
                Ok(byte) => byte,
                Err(_) => return Ok(None),
            };
        }
        decode(&payload).map(Some).map_err(|_| Failure::ReadRequest)
    })
    .await
    .map_err(|_| Failure::ReadRequest)?
}

pub async fn response(stream: &mut UnixStream, response: &Answer) -> Result<(), Failure> {
    let bytes = encode(response).map_err(|_| Failure::InvalidReply)?;
    tokio::time::timeout_at(deadline(), async {
        stream.write_all(&bytes).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| Failure::InvalidReply)?
    .map_err(|_| Failure::InvalidReply)
}
