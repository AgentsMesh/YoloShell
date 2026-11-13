use crate::{constants::*, execute, watch_owner};
use anyhow::{Result, bail};
use std::os::fd::{AsRawFd, BorrowedFd};
use yolo_hub_file_helper_protocol::{MAX_HELPER_LIFETIME, Request, Response};
use yolo_hub_terminal_io::Descriptor;
use yolo_hub_transport::{Framed, Incoming};

pub async fn run() -> Result<()> {
    watch_owner()?;
    let descriptor = unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) }.try_clone_to_owned()?;
    let socket = std::os::unix::net::UnixStream::from(descriptor);
    socket.peer_addr()?;
    let mut channel = Framed::new(Descriptor::duplicate(socket.as_raw_fd())?);
    let deadline = tokio::time::Instant::now() + MAX_HELPER_LIFETIME;
    let Incoming::Message(request) = channel.receive::<Request>(deadline).await? else {
        bail!(EXPECTED_REQUEST)
    };
    let response = Response::new(&request, execute(&request));
    response.validate_for(&request)?;
    channel.send(&response, deadline).await?;
    Ok(())
}
