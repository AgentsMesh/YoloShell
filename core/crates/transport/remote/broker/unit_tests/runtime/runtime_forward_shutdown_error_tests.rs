use super::super::runtime_forward::shutdown_local;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

const SHUTDOWN_ERROR_MESSAGE: &str = "local shutdown failed";

struct FailingShutdownWriter(io::ErrorKind);

impl AsyncWrite for FailingShutdownWriter {
    fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, _: &[u8]) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::Error::new(self.0, SHUTDOWN_ERROR_MESSAGE)))
    }
}

#[tokio::test]
async fn local_shutdown_propagates_the_inner_io_error() {
    let error = shutdown_local(&mut FailingShutdownWriter(io::ErrorKind::Other))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), SHUTDOWN_ERROR_MESSAGE);
}

#[tokio::test]
async fn terminal_local_shutdown_errors_are_already_complete() {
    for kind in [
        io::ErrorKind::BrokenPipe,
        io::ErrorKind::ConnectionAborted,
        io::ErrorKind::ConnectionReset,
        io::ErrorKind::NotConnected,
    ] {
        shutdown_local(&mut FailingShutdownWriter(kind))
            .await
            .unwrap();
    }
}
