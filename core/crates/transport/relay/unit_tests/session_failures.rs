use super::*;
use crate::protocol::error_code::INTERNAL;
use crate::relay::session_test_support::{
    PendingInput, closed_resize_channel, empty_attach, options, owned_prefix,
    owned_with_attachment, snapshot_attach,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::io::Cursor;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const INPUT: &[u8] = b"rejected mutation";
const SNAPSHOT: &[u8] = b"initial snapshot";
const INJECTED_IO_ERROR: &str = "injected relay I/O failure";

struct FailingInput;

impl AsyncRead for FailingInput {
    fn poll_read(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        _buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(std::io::Error::other(INJECTED_IO_ERROR)))
    }
}

struct FailingOutput {
    fail_write: bool,
    successful_flushes: usize,
}

impl AsyncWrite for FailingOutput {
    fn poll_write(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.fail_write {
            Poll::Ready(Err(std::io::Error::other(INJECTED_IO_ERROR)))
        } else {
            Poll::Ready(Ok(buffer.len()))
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.successful_flushes == 0 {
            self.successful_flushes += 1;
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(std::io::Error::other(INJECTED_IO_ERROR)))
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

async fn relay_with(
    terminal_id: Uuid,
    host: &FakeHost,
    input: impl AsyncRead + Unpin + Send,
) -> Result<RelayExit, RelayError> {
    relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
}

#[tokio::test]
async fn non_ownership_mutation_error_is_not_swallowed_as_fencing() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.extend([Reply::error(INTERNAL), Reply::value(&())]);
    let host = FakeHost::start(replies).await;
    assert!(matches!(
        relay_with(terminal_id, &host, Cursor::new(INPUT)).await,
        Err(RelayError::Client(_))
    ));
}

#[tokio::test]
async fn malformed_attachment_frame_is_not_treated_as_transport_recovery() {
    let terminal_id = Uuid::new_v4();
    let mut replies =
        owned_with_attachment(terminal_id, Reply::malformed_attachment(empty_attach()));
    replies.push(Reply::value(&()));
    let host = FakeHost::start(replies).await;
    assert!(matches!(
        relay_with(terminal_id, &host, PendingInput).await,
        Err(RelayError::Client(_))
    ));
}

#[tokio::test]
async fn input_read_failure_releases_the_writer_and_preserves_io_error() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.push(Reply::value(&()));
    let host = FakeHost::start(replies).await;
    assert!(matches!(
        relay_with(terminal_id, &host, FailingInput).await,
        Err(RelayError::Io(_))
    ));
}

#[tokio::test]
async fn initial_write_and_final_flush_failures_are_propagated() {
    for (initial, fail_write) in [(snapshot_attach(SNAPSHOT), true), (empty_attach(), false)] {
        let terminal_id = Uuid::new_v4();
        let mut replies = owned_with_attachment(terminal_id, Reply::attachment(initial, Vec::new()));
        replies.push(Reply::value(&()));
        let host = FakeHost::start(replies).await;
        let result = relay_session(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            tokio::io::empty(),
            FailingOutput {
                fail_write,
                successful_flushes: 0,
            },
            closed_resize_channel(),
        )
        .await;
        assert!(matches!(result, Err(RelayError::Io(_))));
    }
}
