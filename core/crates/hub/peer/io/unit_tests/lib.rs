use crate::PeerIo;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::io::AsyncWriteExt;

const ONE: usize = 1;
const BUFFER_BYTES: usize = 64;

#[tokio::test]
async fn drop_and_explicit_close_run_the_transport_close_exactly_once() {
    let closes = Arc::new(AtomicUsize::default());
    let observed = closes.clone();
    let (stream, _remote) = tokio::io::duplex(BUFFER_BYTES);
    let io = PeerIo::new(stream, move || {
        observed.fetch_add(ONE, Ordering::SeqCst);
    });
    io.close();
    io.close();
    assert_eq!(closes.load(Ordering::SeqCst), ONE);
    drop(io);
    assert_eq!(closes.load(Ordering::SeqCst), ONE);
}

#[tokio::test]
async fn a_closed_transport_refuses_writes() {
    let (stream, _remote) = tokio::io::duplex(BUFFER_BYTES);
    let io = PeerIo::new(stream, || {});
    io.close();
    let mut io = io;
    assert!(io.write_all(b"x").await.is_err());
}
