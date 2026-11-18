use crate::constants::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, split};
use tokio::time::{Instant, interval};
use yolo_hub_dispatch::TerminalSource;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_terminal_protocol::{FIRST_OUTPUT_SEQUENCE, OutputCursor, PublicError, TerminalEvent};
use yolo_hub_state::HubState;

pub(crate) async fn serve<S: AsyncRead + AsyncWrite + Send + Unpin>(
    stream: S,
    mut source: Box<dyn TerminalSource>,
    state: Arc<HubState>,
) -> Result<()> {
    let (mut reader, writer) = split(stream);
    let mut writer = Framed::new(writer);
    let stop = async {
        match reader.read_u8().await {
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(()),
            Err(error) => Err(error.into()),
            Ok(_) => anyhow::bail!(UNEXPECTED_ATTACH_INPUT),
        }
    };
    let result = tokio::select! {
        result = forward(&mut writer, source.as_mut(), &state) => result,
        result = stop => result,
    };
    // Dropping a cancelled attachment closes its read channel; it never stops the PTY.
    let _ = writer.close(Instant::now() + SHUTDOWN_TIMEOUT).await;
    result
}

async fn forward<S: AsyncWrite + Unpin>(
    writer: &mut Framed<S>,
    source: &mut dyn TerminalSource,
    state: &HubState,
) -> Result<()> {
    let initial = source.initial().fields();
    let mut cursor = OutputCursor {
        target: initial.target,
        after_sequence: initial.next_sequence - FIRST_OUTPUT_SEQUENCE,
    };
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    loop {
        let next = source.next_event();
        tokio::pin!(next);
        let event = loop {
            tokio::select! {
                event = &mut next => break event?,
                _ = heartbeat.tick() => {
                    state.verify()?;
                    yolo_hub_terminal_wire::send_event(writer, &TerminalEvent::Heartbeat { cursor }, Instant::now() + OPERATION_TIMEOUT).await?;
                }
            }
        };
        state.verify()?;
        match event.advance(cursor) {
            Ok(current) => cursor = current,
            Err(PublicError::OutputGap)
                if matches!(event, TerminalEvent::ResyncRequired { .. }) => {}
            Err(PublicError::Unreachable) if matches!(event, TerminalEvent::Unreachable { .. }) => {
            }
            Err(error) => return Err(error.into()),
        }
        yolo_hub_terminal_wire::send_event(writer, &event, Instant::now() + OPERATION_TIMEOUT)
            .await?;
        if event.is_terminal() {
            return Ok(());
        }
    }
}
