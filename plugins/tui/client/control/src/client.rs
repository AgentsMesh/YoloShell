use crate::{constants::*, endpoint::DescriptorRead, framing, peer};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use tokio::time::{Instant, timeout_at};
use yolo_local_unix::LocalUnixStream;
use yolo_tui_client_control_protocol::{
    CONTROL_PROTOCOL_VERSION, ControlCommand, ControlRequest, ControlResponse,
};

/// Carries one authenticated owner's descriptor identity through a later detach.
/// An inactive client directory reused by another TUI cannot retarget this handle.
pub struct ClientHandle {
    descriptor: DescriptorRead,
    socket: PathBuf,
}

impl ClientHandle {
    fn open(root: &Path) -> Result<Self> {
        let descriptor = DescriptorRead::open(root)?;
        let socket = yolo_paths::Paths::new(root).worker_socket(descriptor.value.endpoint)?;
        Ok(Self { descriptor, socket })
    }

    pub async fn detach(&self) -> Result<()> {
        let response = self.exchange(ControlCommand::Detach).await?;
        ensure!(response.confirms_detached(), CONTROL_DETACH_REJECTED);
        Ok(())
    }

    async fn exchange(&self, command: ControlCommand) -> Result<ControlResponse> {
        let deadline = Instant::now() + CONTROL_REQUEST_TIMEOUT;
        self.descriptor.verify()?;
        let mut stream = timeout_at(deadline, LocalUnixStream::connect(&self.socket))
            .await
            .context(CONTROL_DEADLINE)??;
        peer::verify(&stream)?;
        self.descriptor.verify()?;
        let request = match command {
            ControlCommand::Detach => ControlRequest::new(self.descriptor.value.token),
            ControlCommand::Status => ControlRequest::status(self.descriptor.value.token),
        };
        framing::send(&mut stream, &request, deadline).await?;
        framing::receive(&mut stream, deadline).await
    }
}

pub async fn detach(root: &Path) -> Result<()> {
    ClientHandle::open(root)?.detach().await
}

/// Authenticated read-only discovery. No control capability is sent to an unverified UID.
pub async fn inspect(root: &Path) -> Result<Option<ClientHandle>> {
    match inspect_current(root).await {
        Err(error)
            if matches!(
                yolo_paths::private_file::io_kind(&error),
                Some(std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused)
            ) =>
        {
            Ok(None)
        }
        result => result,
    }
}

async fn inspect_current(root: &Path) -> Result<Option<ClientHandle>> {
    let handle = ClientHandle::open(root)?;
    let response = handle.exchange(ControlCommand::Status).await?;
    ensure!(
        response.version == CONTROL_PROTOCOL_VERSION && response.command == ControlCommand::Status,
        CONTROL_PROTOCOL_ERROR
    );
    Ok(response.accepted.then_some(handle))
}

pub async fn status(root: &Path) -> Result<bool> {
    Ok(inspect(root).await?.is_some())
}
