mod admission;

use crate::PeerIo;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BUFFER_BYTES: usize = 1;
const COUNT_ONE: usize = 1;
const DATA: &[u8] = b"blocked";
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[tokio::test]
async fn close_wakes_both_pending_io_directions_and_runs_custody_only_once() {
    let calls = Arc::new(AtomicUsize::default());
    let observed = calls.clone();
    let (stream, _remote) = tokio::io::duplex(BUFFER_BYTES);
    let (stream, custody) = PeerIo::new(stream, move || {
        observed.fetch_add(COUNT_ONE, Ordering::SeqCst);
    })
    .into_parts();
    let (mut read, mut write) = tokio::io::split(stream);
    let reading = tokio::spawn(async move { read.read_u8().await });
    let writing = tokio::spawn(async move { write.write_all(DATA).await });
    tokio::task::yield_now().await;
    custody.close();
    custody.close();
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
async fn explicit_transport_close_fences_write_flush_and_shutdown_without_reentering_owner() {
    let calls = Arc::new(AtomicUsize::default());
    let observed = calls.clone();
    let (stream, mut remote) = tokio::io::duplex(DATA.len());
    let (mut stream, custody) = PeerIo::new(stream, move || {
        observed.fetch_add(COUNT_ONE, Ordering::SeqCst);
    })
    .into_parts();
    stream.write_all(DATA).await.unwrap();
    stream.flush().await.unwrap();
    let mut output = vec![u8::default(); DATA.len()];
    remote.read_exact(&mut output).await.unwrap();
    assert_eq!(output, DATA);
    custody.close();
    assert_eq!(
        stream.write(DATA).await.unwrap_err().kind(),
        std::io::ErrorKind::NotConnected
    );
    assert_eq!(
        stream.flush().await.unwrap_err().kind(),
        std::io::ErrorKind::NotConnected
    );
    stream.shutdown().await.unwrap();
    drop((stream, custody));
    assert_eq!(calls.load(Ordering::SeqCst), COUNT_ONE);
}

#[test]
fn dropping_an_unadmitted_transport_releases_its_owner_once() {
    let calls = Arc::new(AtomicUsize::default());
    let observed = calls.clone();
    let (stream, _) = tokio::io::duplex(BUFFER_BYTES);
    drop(PeerIo::new(stream, move || {
        observed.fetch_add(COUNT_ONE, Ordering::SeqCst);
    }));
    assert_eq!(calls.load(Ordering::SeqCst), COUNT_ONE);
}
