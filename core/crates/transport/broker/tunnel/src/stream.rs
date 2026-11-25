use crate::BrokerTunnelClose;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use yolo_broker_hub_protocol::{BrokerHubConnectionBinding, BrokerHubOpened};
use yolo_local_unix::LocalUnixStream;

const CLOSED: &str = "broker Hub tunnel is closed";

pub struct BrokerHubTunnel {
    stream: LocalUnixStream,
    opened: BrokerHubOpened,
    close: BrokerTunnelClose,
}

impl BrokerHubTunnel {
    pub fn new(stream: LocalUnixStream, opened: BrokerHubOpened) -> io::Result<Self> {
        opened
            .request
            .validate()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let close = BrokerTunnelClose::capture(&stream)?;
        Ok(Self {
            stream,
            opened,
            close,
        })
    }
    pub fn binding(&self) -> &BrokerHubConnectionBinding {
        &self.opened.request.binding
    }
    pub fn opened(&self) -> &BrokerHubOpened {
        &self.opened
    }
    pub fn close_handle(&self) -> BrokerTunnelClose {
        self.close.clone()
    }
    pub fn close(&self) {
        self.close.close();
    }
}

impl Drop for BrokerHubTunnel {
    fn drop(&mut self) {
        self.close();
    }
}

impl AsyncRead for BrokerHubTunnel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.close.0.reader.register(context.waker());
        if self.close.closed() {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.stream).poll_read(context, output)
    }
}

impl AsyncWrite for BrokerHubTunnel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.close.0.writer.register(context.waker());
        if self.close.closed() {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.stream).poll_write(context, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.close.0.writer.register(context.waker());
        if self.close.closed() {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.stream).poll_flush(context)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.close.0.writer.register(context.waker());
        if self.close.closed() {
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.stream).poll_shutdown(context)
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, CLOSED)
}
