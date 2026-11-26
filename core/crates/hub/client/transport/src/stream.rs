use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::OwnedSemaphorePermit,
};
use uuid::Uuid;
use yolo_hub_client_access_registry::ClientAdmission;

pub trait Duplex: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Duplex for T {}

pub struct IncomingChannel {
    pub admission: ClientAdmission,
    pub connection_id: Uuid,
    pub stream: ClientStream,
}

pub struct ClientStream {
    stream: Box<dyn Duplex>,
    _permit: OwnedSemaphorePermit,
}
impl ClientStream {
    pub(crate) fn new(stream: impl Duplex + 'static, permit: OwnedSemaphorePermit) -> Self {
        Self {
            stream: Box::new(stream),
            _permit: permit,
        }
    }
}
impl AsyncRead for ClientStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(context, output)
    }
}
impl AsyncWrite for ClientStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(context, input)
    }
    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(context)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(context)
    }
}
