use super::constants::*;
use crate::{CliError, status_descriptor};
use std::os::fd::{AsRawFd, IntoRawFd};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn inherited_command_socket_is_cloexec_bidirectional_and_closed_once() {
    let (child, parent) = std::os::unix::net::UnixStream::pair().unwrap();
    parent.set_nonblocking(true).unwrap();
    let fd = child.into_raw_fd();
    let mut adopted = status_descriptor::adopt(fd).unwrap();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    assert_ne!(flags & libc::FD_CLOEXEC, i32::default());
    let mut parent = tokio::net::UnixStream::from_std(parent).unwrap();
    parent.write_all(INVALID_FRAME).await.unwrap();
    let mut received = vec![0; INVALID_FRAME.len()];
    adopted.read_exact(&mut received).await.unwrap();
    assert_eq!(received, INVALID_FRAME);
    adopted.write_all(INVALID_FRAME).await.unwrap();
    parent.read_exact(&mut received).await.unwrap();
    assert_eq!(received, INVALID_FRAME);
    drop(adopted);
    assert_eq!(parent.read(&mut received).await.unwrap(), usize::default());
}

#[tokio::test]
async fn duplicate_descriptors_are_rejected_before_either_socket_is_adopted() {
    let (left, _right) = std::os::unix::net::UnixStream::pair().unwrap();
    let args = crate::Arguments {
        socket: SOCKET.into(),
        hub: HUB.parse().unwrap(),
        origin: ORIGIN.parse().unwrap(),
        terminal_id: SESSION.parse().unwrap(),
        runtime_epoch: EPOCH.parse().unwrap(),
        holder_id: HOLDER.parse().unwrap(),
        status_fd: Some(left.as_raw_fd()),
        command_fd: Some(left.as_raw_fd()),
        launch_id: Some(uuid::Uuid::new_v4()),
        observe: true,
        take_control: false,
    };
    assert!(matches!(
        crate::command_io::CommandIo::adopt(&args),
        Err(CliError::CommandDescriptor)
    ));
    assert!(unsafe { libc::fcntl(left.as_raw_fd(), libc::F_GETFD) } >= i32::default());
}
