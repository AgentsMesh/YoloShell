use super::super::constants::*;
use super::{Probe, capture, deadline};
#[path = "tui_clipboard_probe_fixture.rs"]
mod fixture;
use fixture::*;
use yolo_runtime::{
    model::{ExitReason, ExitRecord, OutputGapPayload},
    protocol::{EventEnvelope, event},
};

#[tokio::test]
async fn idle_clipboard_assertions_do_not_stop_reading_a_burst_larger_than_the_old_queue() {
    let mut fixture = Fixture::start().await;
    for sequence in FIRST_SEQUENCE..=BURST_CHUNKS {
        fixture.output(sequence, OUTPUT_BYTE.to_vec()).await;
    }
    fixture
        .observed(|capture| capture.sequence == Some(BURST_CHUNKS))
        .await;
    assert_eq!(
        fixture.probe.capture.lock().unwrap().chunks,
        BURST_CHUNKS as usize
    );
    fixture.probe.check_commands(&[]).await.unwrap();
    // An OSC52 command split across event boundaries remains byte-exact after
    // the idle burst; no snapshot or reattachment repairs missing evidence.
    for (offset, byte) in EXPECTED_COMMAND.iter().enumerate() {
        fixture
            .output(BURST_CHUNKS + FIRST_SEQUENCE + offset as u64, vec![*byte])
            .await;
    }
    fixture
        .probe
        .check_commands(&[EXPECTED_COMMAND])
        .await
        .unwrap();
}

#[tokio::test]
async fn explicit_gap_and_driver_exit_are_reported_before_the_following_eof() {
    for exited in [false, true] {
        let mut fixture = Fixture::start().await;
        fixture.output(FIRST_SEQUENCE, OUTPUT_BYTE.to_vec()).await;
        let message = if exited {
            EventEnvelope::new(
                event::SESSION_EXITED,
                Some(fixture.session.to_string()),
                Some(fixture.epoch.to_string()),
                &ExitRecord {
                    runtime_epoch: fixture.epoch,
                    code: Some(0),
                    signal: None,
                    reason: ExitReason::Natural,
                    observed_at_ms: yolo_runtime::model::now_ms(),
                    last_output_sequence: FIRST_SEQUENCE,
                },
            )
            .unwrap()
        } else {
            EventEnvelope::new(
                event::OUTPUT_GAP,
                Some(fixture.session.to_string()),
                Some(fixture.epoch.to_string()),
                &OutputGapPayload {
                    skipped: FIRST_SEQUENCE,
                    resync_required: true,
                },
            )
            .unwrap()
        };
        tokio::time::timeout_at(deadline(), fixture.peer.event(&message))
            .await
            .unwrap()
            .unwrap();
        fixture.observed(|capture| capture.healthy().is_err()).await;
        let expected = if exited { CAPTURE_EXITED } else { CAPTURE_GAP };
        assert_eq!(fixture.probe.check_commands(&[]).await, Err(expected));
        let evidence = fixture.probe.capture.lock().unwrap().evidence();
        assert!(evidence.contains(&format!("last_sequence=Some({FIRST_SEQUENCE})")));
        fixture.closed().await;
    }
}

#[tokio::test]
async fn unexpected_eof_remains_a_failure_even_after_expected_clipboard_was_observed() {
    let mut fixture = Fixture::start().await;
    fixture
        .output(FIRST_SEQUENCE, EXPECTED_COMMAND.to_vec())
        .await;
    fixture
        .observed(|capture| capture.sequence == Some(FIRST_SEQUENCE))
        .await;
    fixture.peer.close().await;
    fixture.observed(|capture| capture.healthy().is_err()).await;
    assert_eq!(
        fixture.probe.check_commands(&[EXPECTED_COMMAND]).await,
        Err(CAPTURE_EOF)
    );
}

#[tokio::test]
async fn capture_byte_limit_fails_closed_without_silently_discarding_outer_output() {
    let mut fixture = Fixture::start().await;
    fixture
        .output(FIRST_SEQUENCE, vec![OUTPUT_BYTE[0]; MAX_CAPTURE_BYTES])
        .await;
    fixture
        .observed(|capture| capture.sequence == Some(FIRST_SEQUENCE))
        .await;
    fixture
        .output(FIRST_SEQUENCE + FIRST_SEQUENCE, OUTPUT_BYTE.to_vec())
        .await;
    fixture.observed(|capture| capture.healthy().is_err()).await;
    assert_eq!(
        fixture.probe.check_commands(&[]).await,
        Err(CAPTURE_OVERFLOW)
    );
    assert!(
        fixture
            .probe
            .capture
            .lock()
            .unwrap()
            .evidence()
            .contains(&format!("buffered_bytes={MAX_CAPTURE_BYTES}"))
    );
    fixture.closed().await;
}
