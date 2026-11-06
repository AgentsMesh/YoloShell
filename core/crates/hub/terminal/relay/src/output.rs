use crate::{Outcome, RelayError, Status, constants::*, control::publish};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::mpsc,
    time::{Instant, timeout},
};
use yolo_hub_terminal_protocol::TerminalEvent;
use yolo_hub_terminal_client::Attachment;

pub(crate) async fn stream<S: AsyncRead + AsyncWrite + Unpin>(
    mut attachment: Attachment<S>,
    mut output: impl AsyncWrite + Unpin,
    status: &mpsc::Sender<Status>,
) -> Result<Outcome, RelayError> {
    let initial = attachment.initial().fields();
    if let Some(snapshot) = &initial.snapshot {
        write(&mut output, &snapshot.fields().data).await?;
    }
    for chunk in &initial.chunks {
        write(&mut output, &chunk.fields().data).await?;
    }
    loop {
        // This future is owned until completion; unrelated control timers cannot cancel it.
        match attachment
            .next_event(Instant::now() + EVENT_TIMEOUT)
            .await?
        {
            TerminalEvent::Output { chunk } => write(&mut output, &chunk.fields().data).await?,
            TerminalEvent::Heartbeat { .. } => {}
            TerminalEvent::Exited { code, signal, .. } => {
                publish(status, Status::SessionExited).await?;
                return Ok(Outcome::Exited { code, signal });
            }
            TerminalEvent::Unreachable { .. } => return Err(RelayError::Unreachable),
            TerminalEvent::ResyncRequired { .. } => return Err(RelayError::ResyncRequired),
        }
    }
}

async fn write(output: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) -> Result<(), RelayError> {
    timeout(OUTPUT_TIMEOUT, async {
        output.write_all(bytes).await?;
        output.flush().await
    })
    .await
    .map_err(|_| RelayError::OutputDeadline)??;
    Ok(())
}
