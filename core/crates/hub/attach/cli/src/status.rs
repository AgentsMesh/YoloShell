use crate::{CliError, constants::*};
use tokio::{
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc,
};
use yolo_hub_terminal_io::Descriptor;
use yolo_hub_terminal_relay::{OUTPUT_TIMEOUT, Status};
use yolo_local_unix::LocalUnixStream;

pub(crate) struct Frame {
    pub event: &'static str,
    pub holder_id: Option<String>,
}
impl From<Status> for Frame {
    fn from(status: Status) -> Self {
        let (event, holder_id) = match status {
            Status::ControlAcquired => (CONTROL_ACQUIRED, None),
            Status::ReadOnly { holder_id } => (READ_ONLY, holder_id),
            Status::ControlLost { holder_id } => (CONTROL_LOST, holder_id),
            Status::SessionExited => (SESSION_EXITED, None),
        };
        Self { event, holder_id }
    }
}

pub(crate) enum Reporter {
    Socket(LocalUnixStream),
    Stderr(Descriptor),
}
impl Reporter {
    pub async fn run(self, mut status: mpsc::Receiver<Status>) -> Result<(), CliError> {
        match self {
            Self::Socket(socket) => {
                let (mut read, mut write) = tokio::io::split(socket);
                let unexpected = async {
                    let mut byte = [0; INBOUND_BYTES];
                    let _ = read.read(&mut byte).await;
                };
                tokio::select! {
                    biased;
                    _ = unexpected => Err(CliError::Status),
                    result = report(&mut write, &mut status) => result,
                }
            }
            Self::Stderr(mut stderr) => {
                while let Some(status) = status.recv().await {
                    let status = Frame::from(status);
                    let line = match status.holder_id { Some(holder) => format!("{}: {holder}\n", status.event), None => format!("{}\n", status.event) };
                    tokio::time::timeout(OUTPUT_TIMEOUT, stderr.write_all(line.as_bytes())).await.map_err(|_| CliError::Status)??;
                }
                Ok(())
            }
        }
    }
}

async fn report(
    output: &mut (impl AsyncWrite + Unpin),
    status: &mut mpsc::Receiver<Status>,
) -> Result<(), CliError> {
    while let Some(status) = status.recv().await {
        use yolo_hub_relay_control_protocol::{Status as NativeStatus, StatusKind};
        let frame = match status {
            Status::ControlAcquired => NativeStatus { kind: StatusKind::ControlAcquired, holder_id: None },
            Status::ReadOnly { holder_id } => NativeStatus { kind: StatusKind::ReadOnly, holder_id },
            Status::ControlLost { holder_id } => NativeStatus { kind: StatusKind::ControlLost, holder_id },
            Status::SessionExited => NativeStatus { kind: StatusKind::SessionExited, holder_id: None },
        };
        let frame = yolo_hub_relay_control_protocol::encode_status(&frame).map_err(|_| CliError::Status)?;
        tokio::time::timeout(OUTPUT_TIMEOUT, async {
            output.write_all(&(frame.len() as u32).to_be_bytes()).await?;
            output.write_all(&frame).await?;
            output.flush().await
        })
        .await
        .map_err(|_| CliError::Status)??;
    }
    Ok(())
}
