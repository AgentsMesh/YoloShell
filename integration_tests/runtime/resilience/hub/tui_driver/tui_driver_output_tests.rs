use super::{Output, capture, deadline};
#[path = "tui_driver_output_fixture.rs"]
mod fixture;
use fixture::*;
use yolo_runtime::{
    model::{ExitReason, ExitRecord, OutputGapPayload},
    protocol::{EventEnvelope, ProtocolError, event},
};

#[tokio::test]
async fn main_driver_drains_bursts_while_no_screen_wait_is_active() {
    let mut fixture = Fixture::start().await;
    for sequence in FIRST_SEQUENCE..=BURST_CHUNKS {
        fixture
            .output(sequence, vec![OUTPUT_BYTE; BURST_BYTES])
            .await;
    }
    fixture
        .observed(|capture| capture.sequence == Some(BURST_CHUNKS))
        .await;
    fixture
        .output(BURST_CHUNKS + FIRST_SEQUENCE, MARKER.to_vec())
        .await;
    let mut screen = vt100::Parser::new(SCREEN_ROWS, SCREEN_COLUMNS, SCROLLBACK);
    fixture
        .output
        .wait(&mut screen, |text| text.contains(READY_TEXT))
        .await
        .unwrap();
    assert_eq!(
        fixture.output.capture.lock().unwrap().chunks,
        (BURST_CHUNKS + FIRST_SEQUENCE) as usize
    );
    fixture.output.close().await;
    assert!(matches!(
        tokio::time::timeout_at(deadline(), fixture.peer.request())
            .await
            .unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}

#[tokio::test]
async fn a_gap_or_driver_exit_is_not_hidden_by_a_matching_cached_screen() {
    for exited in [false, true] {
        let mut fixture = Fixture::start().await;
        fixture.output(FIRST_SEQUENCE, MARKER.to_vec()).await;
        let mut screen = vt100::Parser::new(SCREEN_ROWS, SCREEN_COLUMNS, SCROLLBACK);
        fixture
            .output
            .wait(&mut screen, |text| text.contains(READY_TEXT))
            .await
            .unwrap();
        assert!(screen.screen().contents().contains(READY_TEXT));
        let message = if exited {
            EventEnvelope::new(
                event::SESSION_EXITED,
                Some(fixture.session.to_string()),
                Some(fixture.epoch.to_string()),
                &ExitRecord {
                    runtime_epoch: fixture.epoch,
                    code: Some(SUCCESS_EXIT_CODE),
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
        fixture.observed(|capture| capture.failure.is_some()).await;
        assert!(
            fixture
                .output
                .wait(&mut screen, |text| text.contains(READY_TEXT))
                .await
                .is_err()
        );
        assert_eq!(
            fixture.output.capture.lock().unwrap().failure,
            Some(if exited {
                capture::Failure::DriverExited
            } else {
                capture::Failure::OutputGap
            })
        );
    }
}

#[tokio::test]
async fn eof_and_the_unchanged_one_mebibyte_pending_bound_remain_failures() {
    let mut fixture = Fixture::start().await;
    fixture.output(FIRST_SEQUENCE, MARKER.to_vec()).await;
    let mut screen = vt100::Parser::new(SCREEN_ROWS, SCREEN_COLUMNS, SCROLLBACK);
    fixture
        .output
        .wait(&mut screen, |text| text.contains(READY_TEXT))
        .await
        .unwrap();
    assert!(screen.screen().contents().contains(READY_TEXT));
    fixture.peer.close().await;
    fixture.observed(|capture| capture.failure.is_some()).await;
    assert!(
        fixture
            .output
            .wait(&mut screen, |text| text.contains(READY_TEXT))
            .await
            .is_err()
    );
    assert_eq!(
        fixture.output.capture.lock().unwrap().failure,
        Some(capture::Failure::EndOfStream)
    );
    let mut fixture = Fixture::start().await;
    fixture
        .output(
            FIRST_SEQUENCE,
            vec![OUTPUT_BYTE; crate::hub::support::MAX_OBSERVED_BYTES],
        )
        .await;
    fixture
        .observed(|capture| capture.sequence == Some(FIRST_SEQUENCE))
        .await;
    fixture
        .output(FIRST_SEQUENCE + FIRST_SEQUENCE, vec![OUTPUT_BYTE])
        .await;
    fixture.observed(|capture| capture.failure.is_some()).await;
    assert_eq!(
        fixture.output.capture.lock().unwrap().failure,
        Some(capture::Failure::PendingBytesExceeded)
    );
}
