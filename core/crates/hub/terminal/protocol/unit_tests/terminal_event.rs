use crate::fixtures::{self, constants::*, reject_field, roundtrip};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::*;

fn cursor() -> OutputCursor {
    OutputCursor {
        target: fixtures::target(),
        after_sequence: TEST_SEQUENCE,
    }
}

#[test]
fn output_advances_once_and_heartbeat_exit_only_confirm_exact_cursor() {
    let prior = cursor();
    let output = TerminalEvent::Output {
        chunk: OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE + STEP)).unwrap(),
    };
    let next = output.advance(prior).unwrap();
    assert_eq!(next.after_sequence, TEST_SEQUENCE + STEP);
    assert!(!output.is_terminal());
    roundtrip(&output);
    let heartbeat = TerminalEvent::Heartbeat { cursor: next };
    assert_eq!(heartbeat.advance(next), Ok(next));
    assert!(!heartbeat.is_terminal());
    roundtrip(&heartbeat);
    for code in [None, Some(0), Some(MAX_EXIT_CODE)] {
        let exited = TerminalEvent::Exited {
            cursor: next,
            code,
            signal: None,
        };
        assert_eq!(exited.advance(next), Ok(next));
        assert!(exited.is_terminal());
        roundtrip(&exited);
    }
    let exited = TerminalEvent::Exited {
        cursor: next,
        code: None,
        signal: Some(STEP as i32),
    };
    assert_eq!(exited.advance(next), Ok(next));
    roundtrip(&exited);
}

#[test]
fn disconnect_is_not_exit_and_gap_never_advances_cursor() {
    for (event, error) in [
        (
            TerminalEvent::ResyncRequired {
                target: fixtures::target(),
            },
            PublicError::OutputGap,
        ),
        (
            TerminalEvent::Unreachable {
                target: fixtures::target(),
            },
            PublicError::Unreachable,
        ),
    ] {
        assert_eq!(event.advance(cursor()), Err(error));
        assert!(event.is_terminal());
        roundtrip(&event);
        reject_field(&event, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
    }
}

#[test]
fn every_event_is_fenced_by_target_and_cursor() {
    let current = cursor();
    let output = TerminalEvent::Output {
        chunk: OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE)).unwrap(),
    };
    assert_eq!(output.advance(current), Err(PublicError::OutputGap));
    let mut other = current;
    other.target.runtime_epoch = RuntimeEpoch::new();
    assert_eq!(output.advance(other), Err(PublicError::TargetMismatch));
    other = current;
    other.after_sequence = u64::MAX;
    assert_eq!(output.advance(other), Err(PublicError::InvalidCursor));
    let mut previous = current;
    previous.after_sequence -= STEP;
    for event in [
        TerminalEvent::Heartbeat { cursor: current },
        TerminalEvent::Exited {
            cursor: current,
            code: None,
            signal: None,
        },
    ] {
        assert_eq!(event.advance(previous), Err(PublicError::OutputGap));
        reject_field(&event, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
    }
    reject_field(&output, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
}

#[test]
fn invalid_exit_details_and_exhausted_cursors_are_rejected_during_decode() {
    for (code, signal) in [
        (Some(-1), None),
        (Some(MAX_EXIT_CODE + STEP as i32), None),
        (None, Some(0)),
        (None, Some(-1)),
        (Some(0), Some(STEP as i32)),
    ] {
        let event = TerminalEvent::Exited {
            cursor: cursor(),
            code,
            signal,
        };
        assert_eq!(event.validate(), Err(PublicError::InvalidPayload));
        assert!(
            serde_json::from_value::<TerminalEvent>(serde_json::to_value(event).unwrap()).is_err()
        );
    }
    let mut exhausted = cursor();
    exhausted.after_sequence = u64::MAX;
    for event in [
        TerminalEvent::Heartbeat { cursor: exhausted },
        TerminalEvent::Exited {
            cursor: exhausted,
            code: None,
            signal: None,
        },
    ] {
        assert_eq!(event.validate(), Err(PublicError::InvalidCursor));
        assert!(
            serde_json::from_value::<TerminalEvent>(serde_json::to_value(event).unwrap()).is_err()
        );
    }
}
