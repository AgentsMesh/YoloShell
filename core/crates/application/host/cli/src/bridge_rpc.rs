use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::Mutex,
};
use yolo_capnp_rpc::{Error, Future, Method, Protocol, Reply, Service, Value};

pub(super) async fn forward<R, W>(
    client: &yolo_client::Client,
    input: R,
    output: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let authenticated = client.connect().await?;
    yolo_capnp_rpc::serve(
        Io { input, output },
        Protocol::Endpoint,
        Arc::new(Bridge(Mutex::new(authenticated))),
    )
    .await?;
    Ok(())
}
struct Bridge(Mutex<yolo_client::RpcConnection>);
impl Service for Bridge {
    fn call(&self, method: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            if method != Method::Call {
                return Err(Error::Invalid);
            }
            let request = yolo_capnp_codec::from_value(value)?;
            self.0
                .lock()
                .await
                .forward(&request)
                .await
                .map(yolo_capnp_rpc::RemoteReply::into_service_reply)
                .map_err(|error| Error::Remote(error.to_string()))
        })
    }
}
struct Io<R, W> {
    input: R,
    output: W,
}
impl<R: AsyncRead + Unpin, W: Unpin> AsyncRead for Io<R, W> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.input).poll_read(cx, buf)
    }
}
impl<R: Unpin, W: AsyncWrite + Unpin> AsyncWrite for Io<R, W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.output).poll_write(cx, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.output).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.output).poll_shutdown(cx)
    }
}
