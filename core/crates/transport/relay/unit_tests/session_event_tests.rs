use super::*;
use crate::model::{ExitReason, ExitRecord};

const EXIT_CODE: i32 = 0;
const EXIT_OBSERVED_AT_MS: i64 = 1;
const EXIT_OUTPUT_SEQUENCE: u64 = 9;
const EXIT_SEQUENCE_OFFSET: u64 = 1;
const IGNORED_EVENT: &str = "ignored";

#[tokio::test]
async fn control_events_and_event_validation_are_strict() {
    let terminal_id = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    let mut output = Capture::default();
    let mut sequence = EXIT_OUTPUT_SEQUENCE;
    for (event_type, expected) in [
        (event::OUTPUT_GAP, EventAction::Reattach),
        (event::SESSION_EXITED, EventAction::SessionExited),
        (event::WRITER_LEASE_ACQUIRED, EventAction::Continue),
    ] {
        let payload = ExitRecord {
            runtime_epoch: epoch,
            code: Some(EXIT_CODE),
            signal: None,
            reason: ExitReason::Natural,
            observed_at_ms: EXIT_OBSERVED_AT_MS,
            last_output_sequence: EXIT_OUTPUT_SEQUENCE,
        };
        let message = if event_type == event::SESSION_EXITED {
            EventEnvelope::new(
                event_type,
                Some(terminal_id.to_string()),
                Some(epoch.to_string()),
                &payload,
            )
        } else {
            EventEnvelope::new(
                event_type,
                Some(terminal_id.to_string()),
                Some(epoch.to_string()),
                &(),
            )
        }
        .unwrap();
        assert_eq!(
            write_event(&mut output, &message, terminal_id, epoch, &mut sequence)
                .await
                .unwrap(),
            expected
        );
    }
    let valid = EventEnvelope::new(
        IGNORED_EVENT,
        Some(terminal_id.to_string()),
        Some(epoch.to_string()),
        &(),
    )
    .unwrap();
    write_event(&mut output, &valid, terminal_id, epoch, &mut sequence)
        .await
        .unwrap();
    let invalid = EventEnvelope::new(IGNORED_EVENT, None, None, &()).unwrap();
    assert!(matches!(
        write_event(&mut output, &invalid, terminal_id, epoch, &mut sequence).await,
        Err(RelayError::Protocol(ProtocolError::EventMismatch))
    ));
    let foreign_chunk = OutputChunk {
        runtime_epoch: Uuid::new_v4(),
        sequence,
        data: Vec::new(),
    };
    let foreign = EventEnvelope::new(
        event::TERMINAL_OUTPUT,
        Some(terminal_id.to_string()),
        Some(epoch.to_string()),
        &foreign_chunk,
    )
    .unwrap();
    assert!(matches!(
        write_event(&mut output, &foreign, terminal_id, epoch, &mut sequence).await,
        Err(RelayError::Protocol(ProtocolError::EventMismatch))
    ));
}

#[tokio::test]
async fn session_exit_requires_the_exact_observed_output_sequence() {
    let terminal_id = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    for exit_sequence in [
        EXIT_OUTPUT_SEQUENCE
            .checked_sub(EXIT_SEQUENCE_OFFSET)
            .unwrap(),
        EXIT_OUTPUT_SEQUENCE
            .checked_add(EXIT_SEQUENCE_OFFSET)
            .unwrap(),
    ] {
        let exit = ExitRecord {
            runtime_epoch: epoch,
            code: Some(EXIT_CODE),
            signal: None,
            reason: ExitReason::Natural,
            observed_at_ms: EXIT_OBSERVED_AT_MS,
            last_output_sequence: exit_sequence,
        };
        let message = EventEnvelope::new(
            event::SESSION_EXITED,
            Some(terminal_id.to_string()),
            Some(epoch.to_string()),
            &exit,
        )
        .unwrap();
        let mut output = Capture::default();
        let mut sequence = EXIT_OUTPUT_SEQUENCE;

        assert!(matches!(
            write_event(&mut output, &message, terminal_id, epoch, &mut sequence).await,
            Err(RelayError::Protocol(ProtocolError::EventMismatch))
        ));
        assert_eq!(sequence, EXIT_OUTPUT_SEQUENCE);
    }
}
