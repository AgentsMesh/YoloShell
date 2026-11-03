use super::constants::{
    CONTROL_LOST_STATUS, CONTROL_STATUS, HOST_STARTED_STATUS, HUMAN_STATUS_WRITE_CONTEXT,
    NDJSON_DELIMITER, READ_ONLY_STATUS, SESSION_ENDING_STATUS, SESSION_EXITED_STATUS,
    SESSION_RESUMED_STATUS, STATUS_PEER_CLOSED, STATUS_PEER_INPUT, STATUS_WRITE_TIMEOUT,
    STATUS_WRITE_TIMEOUT_MESSAGE, UNKNOWN_HOLDER_STATUS,
};
use super::status_protocol::StatusFrame;
use anyhow::{Context, Result, bail};
use std::os::fd::RawFd;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use yolo_runtime::local_unix::LocalUnixStream;
use yolo_relay::RelayStatus;

#[path = "status_descriptor.rs"]
mod descriptor;

pub(super) enum StatusReporter {
    Human,
    Socket(LocalUnixStream),
}

impl StatusReporter {
    pub(super) fn from_inherited_fd(descriptor: Option<RawFd>) -> Result<Self> {
        match descriptor {
            Some(descriptor) => descriptor::adopt(descriptor).map(Self::Socket),
            None => Ok(Self::Human),
        }
    }

    pub(super) async fn run(self, mut events: mpsc::UnboundedReceiver<RelayStatus>) -> Result<()> {
        match self {
            Self::Human => run_human(tokio::io::stderr(), &mut events).await,
            Self::Socket(stream) => run_socket(stream, &mut events).await,
        }
    }
}

async fn run_human(
    mut output: impl tokio::io::AsyncWrite + Unpin,
    events: &mut mpsc::UnboundedReceiver<RelayStatus>,
) -> Result<()> {
    while let Some(status) = events.recv().await {
        let mut line = status_message(status).into_bytes();
        line.push(NDJSON_DELIMITER);
        tokio::time::timeout(STATUS_WRITE_TIMEOUT, output.write_all(&line))
            .await
            .context(STATUS_WRITE_TIMEOUT_MESSAGE)?
            .context(HUMAN_STATUS_WRITE_CONTEXT)?;
    }
    Ok(())
}

async fn run_socket(
    stream: LocalUnixStream,
    events: &mut mpsc::UnboundedReceiver<RelayStatus>,
) -> Result<()> {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let mut inbound = [0_u8; 1];
    loop {
        tokio::select! {
            read = reader.read(&mut inbound) => match read? {
                0 => bail!(STATUS_PEER_CLOSED),
                _ => bail!(STATUS_PEER_INPUT),
            },
            status = events.recv() => {
                let Some(status) = status else { return Ok(()); };
                let frame = StatusFrame::from(status);
                let line = yolo_hub_relay_control_protocol::encode_status(&frame.native())?;
                let mut bytes = (line.len() as u32).to_be_bytes().to_vec();
                bytes.extend_from_slice(&line);
                tokio::select! {
                    read = reader.read(&mut inbound) => match read? {
                        0 => bail!(STATUS_PEER_CLOSED),
                        _ => bail!(STATUS_PEER_INPUT),
                    },
                    write = tokio::time::timeout(
                        STATUS_WRITE_TIMEOUT,
                        writer.write_all(&bytes),
                    ) => {
                        write.context(STATUS_WRITE_TIMEOUT_MESSAGE)??;
                    }
                }
            }
        }
    }
}

pub(super) fn status_message(status: RelayStatus) -> String {
    match status {
        RelayStatus::ControlAcquired => CONTROL_STATUS.into(),
        RelayStatus::ReadOnly { holder_id } => format!(
            "{READ_ONLY_STATUS}; holder={}",
            holder_id.as_deref().unwrap_or(UNKNOWN_HOLDER_STATUS)
        ),
        RelayStatus::ControlLost { holder_id } => format!(
            "{CONTROL_LOST_STATUS}; holder={}",
            holder_id.as_deref().unwrap_or(UNKNOWN_HOLDER_STATUS)
        ),
        RelayStatus::HostStarted { executable } => {
            format!("{HOST_STARTED_STATUS}: {}", executable.display())
        }
        RelayStatus::SessionResumed => SESSION_RESUMED_STATUS.into(),
        RelayStatus::SessionEnding => SESSION_ENDING_STATUS.into(),
        RelayStatus::SessionExited => SESSION_EXITED_STATUS.into(),
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-test-support"))] #[path = "../test_support/status_ancillary.rs"]
mod ancillary_test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/status_security_tests.rs"]
mod security_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/status_tests.rs"]
mod tests;
