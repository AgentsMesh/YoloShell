use crate::CliError;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use yolo_hub_relay_control_protocol::{
    MAX_FRAME_BYTES, Request, Response, decode_request, encode_response,
};
use yolo_hub_terminal_relay::{OUTPUT_TIMEOUT, REQUEST_TIMEOUT};

pub(crate) async fn read(input: &mut (impl AsyncRead + Unpin)) -> Result<Request, CliError> {
    // Idle connections have no deadline. Once a frame begins, its bounded
    // completion deadline prevents a partial sender holding the channel forever.
    let first = input.read_u8().await.map_err(|_| CliError::Command)?;
    tokio::time::timeout(REQUEST_TIMEOUT, async {
        let mut prefix = [0; yolo_hub_relay_control_protocol::FRAME_PREFIX_BYTES];
        prefix[0] = first;
        input.read_exact(&mut prefix[1..]).await.map_err(|_| CliError::Command)?;
        let length = u32::from_be_bytes(prefix) as usize;
        if length == 0 || length > MAX_FRAME_BYTES { return Err(CliError::Command); }
        let mut bytes = vec![0; length];
        input.read_exact(&mut bytes).await.map_err(|_| CliError::Command)?;
        decode_request(&bytes).map_err(|_| CliError::Command)
    })
    .await
    .map_err(|_| CliError::Command)?
}

pub(crate) async fn write(
    output: &mut (impl AsyncWrite + Unpin),
    response: &Response,
) -> Result<(), CliError> {
    let bytes = encode_response(response).map_err(|_| CliError::Command)?;
    tokio::time::timeout(OUTPUT_TIMEOUT, async {
        output.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
        output.write_all(&bytes).await?;
        output.flush().await
    })
    .await
    .map_err(|_| CliError::Command)??;
    Ok(())
}
