use crate::{FixtureError, constants::*};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use yolo_hub_connector_protocol::Response;

pub(crate) async fn next<R: AsyncBufRead + Unpin>(
    input: &mut R,
) -> Result<Option<Vec<u8>>, FixtureError> {
    let mut line = Vec::new();
    loop {
        let available = input.fill_buf().await.map_err(|_| FixtureError::Input)?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(FixtureError::InvalidRequest)
            };
        }
        let end = available.iter().position(|byte| *byte == LINE_END);
        let length = end.unwrap_or(available.len());
        if length > MAX_LINE_BYTES.saturating_sub(line.len()) {
            return Err(FixtureError::TooLarge);
        }
        line.extend_from_slice(&available[..length]);
        input.consume(length + usize::from(end.is_some()) * DELIMITER_BYTES);
        if end.is_some() {
            return Ok(Some(line));
        }
    }
}

pub(crate) async fn write<W: AsyncWrite + Unpin>(
    output: &mut W,
    response: &Response,
) -> Result<(), FixtureError> {
    let bytes = serde_json::to_vec(response).map_err(|_| FixtureError::Output)?;
    if bytes.len() > MAX_LINE_BYTES {
        return Err(FixtureError::TooLarge);
    }
    output
        .write_all(&bytes)
        .await
        .map_err(|_| FixtureError::Output)?;
    output
        .write_all(LINE_TERMINATOR)
        .await
        .map_err(|_| FixtureError::Output)?;
    output.flush().await.map_err(|_| FixtureError::Output)
}
