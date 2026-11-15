use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn close_wakes_both_pending_directions_and_calls_owner_once() {
    let calls = Arc::new(AtomicUsize::default());
    let observed = calls.clone();
    let (stream, _remote) = tokio::io::duplex(BUFFER_BYTES);
    let (stream, custody) = ClientIo::new(stream, move || {
        observed.fetch_add(COUNT_ONE, Ordering::SeqCst);
    })
    .into_parts();
    let (mut read, mut write) = tokio::io::split(stream);
    let reading = tokio::spawn(async move { read.read_u8().await });
    let writing = tokio::spawn(async move { write.write_all(DATA).await });
    tokio::task::yield_now().await;
    custody.0.close();
    custody.0.close();
    assert_eq!(
        tokio::time::timeout(TIMEOUT, reading)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotConnected
    );
    assert_eq!(
        tokio::time::timeout(TIMEOUT, writing)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotConnected
    );
    drop(custody);
    assert_eq!(calls.load(Ordering::SeqCst), COUNT_ONE);
}

#[tokio::test]
async fn closed_transport_fences_write_flush_shutdown_and_read() {
    let calls = Arc::new(AtomicUsize::default());
    let observed = calls.clone();
    let (stream, mut remote) = tokio::io::duplex(DATA.len());
    let (mut stream, custody) = ClientIo::new(stream, move || {
        observed.fetch_add(COUNT_ONE, Ordering::SeqCst);
    })
    .into_parts();
    stream.write_all(DATA).await.unwrap();
    stream.flush().await.unwrap();
    let mut output = vec![u8::default(); DATA.len()];
    remote.read_exact(&mut output).await.unwrap();
    assert_eq!(output, DATA);
    custody.0.close();
    assert_eq!(
        stream.write(DATA).await.unwrap_err().kind(),
        std::io::ErrorKind::NotConnected
    );
    assert_eq!(
        stream.flush().await.unwrap_err().kind(),
        std::io::ErrorKind::NotConnected
    );
    stream.shutdown().await.unwrap();
    assert_eq!(
        stream.read_u8().await.unwrap_err().kind(),
        std::io::ErrorKind::NotConnected
    );
    drop((stream, custody));
    assert_eq!(calls.load(Ordering::SeqCst), COUNT_ONE);
}

#[tokio::test]
async fn close_notification_covers_early_and_late_waiters() {
    let (stream, _) = tokio::io::duplex(BUFFER_BYTES);
    let (_, custody) = ClientIo::new(stream, || {}).into_parts();
    let early = custody.0.clone();
    let waiting = tokio::spawn(async move {
        early.closed().await;
    });
    tokio::task::yield_now().await;
    custody.0.close();
    tokio::time::timeout(TIMEOUT, waiting)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(TIMEOUT, custody.0.closed())
        .await
        .unwrap();
}
