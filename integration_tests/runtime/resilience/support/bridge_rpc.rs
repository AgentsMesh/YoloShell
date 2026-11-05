use super::*;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub(crate) async fn bridge_request(
    child: &mut Child,
    request: &RequestEnvelope,
) -> ResponseEnvelope {
    let io = Pipes {
        input: tokio::process::ChildStdout::from_std(child.stdout.take().unwrap()).unwrap(),
        output: tokio::process::ChildStdin::from_std(child.stdin.take().unwrap()).unwrap(),
    };
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    let mut channel = yolo_runtime_rpc::RuntimeConnection::open(io, deadline)
        .await
        .unwrap();
    channel.exchange(request, deadline).await.unwrap()
}

struct Pipes {
    input: tokio::process::ChildStdout,
    output: tokio::process::ChildStdin,
}
impl AsyncRead for Pipes {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.input).poll_read(cx, bytes)
    }
}
impl AsyncWrite for Pipes {
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
