use super::*;
use crate::model::{OutputChunk, TerminalSnapshot};
use crate::protocol::{EventEnvelope, ProtocolError, event};
use crate::relay::constants::TERMINAL_RESET;
use crate::relay::output::{EventAction, write_event, write_initial};
use crate::worker::AttachResult;
use std::pin::Pin;
use std::task::{Context, Poll};

const INITIAL_OUTPUT_SEQUENCE: u64 = 0;
const SNAPSHOT_BARRIER_SEQUENCE: u64 = 4;
const FIRST_CHUNK_SEQUENCE: u64 = 5;
const NEXT_CHUNK_SEQUENCE: u64 = 6;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const STALE_SNAPSHOT_OUTPUT: &[u8] = b"stale snapshot";

#[derive(Default)]
struct Capture(Vec<u8>);

impl AsyncWrite for Capture {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.0.extend_from_slice(buffer);
        Poll::Ready(Ok(buffer.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn output_event(terminal_id: Uuid, chunk: &OutputChunk) -> EventEnvelope {
    EventEnvelope::new(
        event::TERMINAL_OUTPUT,
        Some(terminal_id.to_string()),
        Some(chunk.runtime_epoch.to_string()),
        chunk,
    )
    .unwrap()
}

#[tokio::test]
async fn initial_output_resets_only_when_a_snapshot_needs_resynchronization() {
    let epoch = Uuid::new_v4();
    let snapshot = TerminalSnapshot {
        runtime_epoch: epoch,
        barrier_sequence: SNAPSHOT_BARRIER_SEQUENCE,
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        data: b"snapshot".to_vec(),
    };
    let mut output = Capture::default();
    let sequence = write_initial(
        &mut output,
        &AttachResult {
            runtime_epoch: epoch,
            snapshot: Some(snapshot),
            chunks: Vec::new(),
            next_sequence: FIRST_CHUNK_SEQUENCE,
            resynchronized: true,
        },
        INITIAL_OUTPUT_SEQUENCE,
        false,
    )
    .await
    .unwrap();
    assert_eq!(sequence, SNAPSHOT_BARRIER_SEQUENCE);
    assert_eq!(output.0, [TERMINAL_RESET, b"snapshot"].concat());

    let chunk = OutputChunk {
        runtime_epoch: epoch,
        sequence: FIRST_CHUNK_SEQUENCE,
        data: b"chunk".to_vec(),
    };
    let mut output = Capture::default();
    let sequence = write_initial(
        &mut output,
        &AttachResult {
            runtime_epoch: epoch,
            snapshot: None,
            chunks: vec![chunk],
            next_sequence: NEXT_CHUNK_SEQUENCE,
            resynchronized: false,
        },
        SNAPSHOT_BARRIER_SEQUENCE,
        true,
    )
    .await
    .unwrap();
    assert_eq!(sequence, FIRST_CHUNK_SEQUENCE);
    assert_eq!(output.0, b"chunk");
}

#[tokio::test]
async fn initial_snapshot_cannot_roll_back_an_observed_cursor() {
    let epoch = Uuid::new_v4();
    let initial = AttachResult {
        runtime_epoch: epoch,
        snapshot: Some(TerminalSnapshot {
            runtime_epoch: epoch,
            barrier_sequence: SNAPSHOT_BARRIER_SEQUENCE,
            cols: TERMINAL_COLUMNS,
            rows: TERMINAL_ROWS,
            data: STALE_SNAPSHOT_OUTPUT.to_vec(),
        }),
        chunks: Vec::new(),
        next_sequence: FIRST_CHUNK_SEQUENCE,
        resynchronized: true,
    };
    let mut output = Capture::default();

    assert!(matches!(
        write_initial(&mut output, &initial, FIRST_CHUNK_SEQUENCE, true).await,
        Err(RelayError::Protocol(ProtocolError::AttachmentMismatch))
    ));
    assert!(output.0.is_empty());
}

#[tokio::test]
async fn live_output_deduplicates_and_reattaches_before_a_sequence_gap() {
    let terminal_id = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    let chunk = |sequence, data: &[u8]| OutputChunk {
        runtime_epoch: epoch,
        sequence,
        data: data.to_vec(),
    };
    let mut output = Capture::default();
    let mut sequence = 0;
    assert_eq!(
        write_event(
            &mut output,
            &output_event(terminal_id, &chunk(1, b"one")),
            terminal_id,
            epoch,
            &mut sequence
        )
        .await
        .unwrap(),
        EventAction::Continue
    );
    assert_eq!(sequence, 1);
    write_event(
        &mut output,
        &output_event(terminal_id, &chunk(1, b"duplicate")),
        terminal_id,
        epoch,
        &mut sequence,
    )
    .await
    .unwrap();
    assert_eq!(
        write_event(
            &mut output,
            &output_event(terminal_id, &chunk(3, b"gap")),
            terminal_id,
            epoch,
            &mut sequence
        )
        .await
        .unwrap(),
        EventAction::Reattach
    );
    assert_eq!(output.0, b"one");
}

#[path = "session_event_tests.rs"]
mod event_tests;
