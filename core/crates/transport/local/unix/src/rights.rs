use super::LocalUnixStream;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd};

pub async fn send_exact(
    stream: &LocalUnixStream,
    marker: u8,
    descriptors: &[BorrowedFd<'_>],
) -> io::Result<()> {
    let raw = descriptors
        .iter()
        .map(AsRawFd::as_raw_fd)
        .collect::<Vec<_>>();
    stream
        .async_io(tokio::io::Interest::WRITABLE, || {
            super::ancillary_send::send_exact(stream.as_raw_fd(), marker, &raw)
        })
        .await
}

pub async fn receive_exact(
    stream: &LocalUnixStream,
    marker: u8,
    descriptor_count: usize,
) -> io::Result<Vec<OwnedFd>> {
    stream
        .async_io(tokio::io::Interest::READABLE, || {
            super::ancillary_recv::receive_exact(stream.as_raw_fd(), marker, descriptor_count)
        })
        .await
}
