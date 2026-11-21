use super::*;
use std::{
    io,
    pin::Pin,
    sync::atomic::AtomicBool,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

struct FailWrites {
    inner: tokio::io::DuplexStream,
    fail: Arc<AtomicBool>,
}
impl AsyncRead for FailWrites {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}
impl AsyncWrite for FailWrites {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.fail.load(Ordering::SeqCst) {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
struct ReceiptLoss {
    fail: Arc<AtomicBool>,
    confirmations: Arc<AtomicUsize>,
}
impl Service for ReceiptLoss {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        let fail = self.fail.clone();
        let count = self.confirmations.clone();
        Box::pin(async move {
            Ok(Reply {
                value,
                events: None,
                on_observed: Some(Box::new(move || {
                    count.fetch_add(1, Ordering::SeqCst);
                    fail.store(true, Ordering::SeqCst);
                })),
            })
        })
    }
}

#[tokio::test]
async fn lost_confirmation_reply_reports_unknown_instead_of_certain_business_success() {
    let (client, inner) = tokio::io::duplex(PIPE_BYTES);
    let fail = Arc::new(AtomicBool::new(false));
    let confirmations = Arc::new(AtomicUsize::new(0));
    let serving = tokio::spawn(yolo_capnp_rpc::serve(
        FailWrites {
            inner,
            fail: fail.clone(),
        },
        Protocol::Endpoint,
        Arc::new(ReceiptLoss {
            fail,
            confirmations: confirmations.clone(),
        }),
    ));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let reply = client
        .request(
            Method::Call,
            Value::Text(PAYLOAD.into()),
            Instant::now() + TIMEOUT,
        )
        .await;
    assert!(matches!(reply, Err(Error::OutcomeUnknown)));
    assert_eq!(confirmations.load(Ordering::SeqCst), 1);
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
}
