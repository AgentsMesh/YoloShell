use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use yolo_local_unix::LocalUnixStream;

pub trait HubEntryIo: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T> HubEntryIo for T where T: AsyncRead + AsyncWrite + Send + Unpin {}

pub enum HubEntryStream {
    Local(LocalUnixStream),
    Remote(Box<dyn HubEntryIo>),
}

impl AsyncRead for HubEntryStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Local(stream) => Pin::new(stream).poll_read(cx, buffer),
            Self::Remote(stream) => Pin::new(stream).poll_read(cx, buffer),
        }
    }
}

impl AsyncWrite for HubEntryStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.as_mut().get_mut() {
            Self::Local(stream) => Pin::new(stream).poll_write(cx, bytes),
            Self::Remote(stream) => Pin::new(stream).poll_write(cx, bytes),
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Local(stream) => Pin::new(stream).poll_flush(cx),
            Self::Remote(stream) => Pin::new(stream).poll_flush(cx),
        }
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Local(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Remote(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
