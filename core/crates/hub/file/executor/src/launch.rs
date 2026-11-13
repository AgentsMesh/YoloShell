use std::{
    os::fd::OwnedFd,
    os::unix::net::UnixStream,
    path::Path,
    process::{Command, Stdio},
};
use tokio::time::Instant;
use yolo_hub_file_helper_protocol::{Payload, Request, Response};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_transport::{Framed, Incoming};
use yolo_launch_contract::ProcessRole;
use yolo_process::ScopedWaitableChild;

pub(crate) async fn call(
    program: &Path,
    request: &Request,
    deadline: Instant,
) -> Result<Payload, PublicError> {
    let (control, child_control) = UnixStream::pair().map_err(|_| PublicError::Unreachable)?;
    let (owner, child_owner) = UnixStream::pair().map_err(|_| PublicError::Unreachable)?;
    control
        .set_nonblocking(true)
        .map_err(|_| PublicError::Unreachable)?;
    let mut channel = Framed::new(
        tokio::net::UnixStream::from_std(control).map_err(|_| PublicError::Unreachable)?,
    );
    let mut command = Command::new(program);
    command
        .args(ProcessRole::FileHelper.arguments())
        .stdin(Stdio::from(OwnedFd::from(child_control)))
        .stdout(Stdio::null())
        .stderr(Stdio::from(OwnedFd::from(child_owner)));
    let child =
        ScopedWaitableChild::spawn_detached_before_deadline(&mut command, deadline.into_std())
            .map_err(|_| PublicError::Unreachable)?;
    drop(command);
    channel
        .send(request, deadline)
        .await
        .map_err(|_| PublicError::Unreachable)?;
    let Incoming::Message(response) = channel
        .receive::<Response>(deadline)
        .await
        .map_err(|_| PublicError::Unreachable)?
    else {
        return Err(PublicError::Unreachable);
    };
    response
        .validate_for(request)
        .map_err(|_| PublicError::Unreachable)?;
    if !matches!(
        channel.receive::<Response>(deadline).await,
        Ok(Incoming::Eof)
    ) {
        return Err(PublicError::Unreachable);
    }
    child
        .wait_for_clean_exit(deadline.into_std())
        .await
        .map_err(|_| PublicError::Unreachable)?;
    drop(owner);
    if Instant::now() >= deadline {
        return Err(PublicError::Unreachable);
    }
    Ok(response.payload)
}
