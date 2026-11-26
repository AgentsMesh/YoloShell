use super::*;
use crate::local_unix::UnexpectedAncillaryData;
use crate::local_unix::test_support::{assert_reader_eof, pipe, send_right};
use std::future::Future;
use std::os::fd::AsRawFd;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

const ANCILLARY_MARKER: u8 = b'P';
const NO_STAGE_POLLS: usize = 0;
const ANCILLARY_REJECTION_EXPECTED: &str = "queued ancillary data was not rejected";

struct ReadyStage {
    polls: Arc<AtomicUsize>,
}

impl Future for ReadyStage {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn queued_right_wins_before_a_ready_runtime_stage_is_polled() {
    let (mut local, peer) = LocalUnixStream::pair().unwrap();
    let transferred = pipe();
    send_right(
        peer.as_raw_fd(),
        transferred.writer.as_raw_fd(),
        ANCILLARY_MARKER,
    );
    drop(transferred.writer);
    let polls = Arc::new(AtomicUsize::new(NO_STAGE_POLLS));
    let mut stage = Box::pin(ReadyStage {
        polls: Arc::clone(&polls),
    });
    let mut prefix = Vec::new();

    let error = match monitor(&mut local, &mut prefix, stage.as_mut()).await {
        Err(error) => error,
        Ok(_) => panic!("{ANCILLARY_REJECTION_EXPECTED}"),
    };

    let transport = error.downcast_ref::<std::io::Error>().unwrap();
    assert!(
        transport
            .get_ref()
            .is_some_and(|cause| cause.is::<UnexpectedAncillaryData>())
    );
    assert_eq!(polls.load(Ordering::SeqCst), NO_STAGE_POLLS);
    assert!(prefix.is_empty());
    assert_reader_eof(&transferred.reader);
}
