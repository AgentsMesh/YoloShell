mod ownership;

use crate::{ClientIo, custody::OpeningGuard};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const COUNT_ONE: usize = 1;
const BUFFER_BYTES: usize = 1;
const DATA: &[u8] = b"pending-public-client-io";
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[test]
fn dropping_unadmitted_transport_releases_owner_once() {
    let calls = Arc::new(AtomicUsize::default());
    let observed = calls.clone();
    let (stream, _) = tokio::io::duplex(BUFFER_BYTES);
    drop(ClientIo::new(stream, move || {
        observed.fetch_add(COUNT_ONE, Ordering::SeqCst);
    }));
    assert_eq!(calls.load(Ordering::SeqCst), COUNT_ONE);
}

#[test]
fn ambiguous_channel_open_retires_transport_but_resolved_rejection_does_not() {
    for interrupted in [true, false] {
        let (stream, _) = tokio::io::duplex(BUFFER_BYTES);
        let (_, custody) = ClientIo::new(stream, || {}).into_parts();
        let mut opening = OpeningGuard(Some(custody.0.clone()));
        if !interrupted {
            opening.0.take();
        }
        drop(opening);
        assert_eq!(custody.0.is_closed(), interrupted);
    }
}

#[test]
fn expiry_clock_returns_checked_epoch_milliseconds() {
    assert!(crate::time::now_ms().unwrap() > u64::default());
}
