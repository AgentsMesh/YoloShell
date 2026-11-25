use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::ExecutionId;

use crate::constants::*;
use crate::fixtures::*;

#[test]
fn cursor_scope_sequence_and_wire_validation() {
    let target = target();
    let stream = Uuid::new_v4();
    let cursor = ObservationCursor::new(target, stream, FIRST_OBSERVATION_SEQUENCE).unwrap();
    assert_eq!(cursor.target(), target);
    assert_eq!(cursor.stream(), stream);
    assert_eq!(cursor.sequence(), FIRST_OBSERVATION_SEQUENCE);
    assert_eq!(round_trip(&cursor), cursor);
    assert_eq!(
        ObservationCursor::new(target, Uuid::nil(), FIRST_OBSERVATION_SEQUENCE),
        Err(ProtocolError::InvalidCursor)
    );
    assert_eq!(
        ObservationCursor::new(target, stream, u64::MIN),
        Err(ProtocolError::InvalidCursor)
    );
    let mut wire = serde_json::to_value(cursor).unwrap();
    wire["sequence"] = u64::MIN.into();
    assert!(serde_json::from_value::<ObservationCursor>(wire).is_err());
}

#[test]
fn replay_detects_duplicate_next_gap_older_and_scope_reset() {
    let cursor = fields().cursor;
    let at = |sequence| ObservationCursor::new(cursor.target(), cursor.stream(), sequence).unwrap();
    assert_eq!(cursor.position_after(cursor), CursorPosition::Duplicate);
    let next = at(cursor.sequence() + SEQUENCE_STEP);
    assert_eq!(next.position_after(cursor), CursorPosition::Next);
    assert_eq!(cursor.position_after(next), CursorPosition::Older);
    assert_eq!(
        at(cursor.sequence() + SEQUENCE_GAP).position_after(cursor),
        CursorPosition::Gap
    );
    let reset = ObservationCursor::new(cursor.target(), Uuid::new_v4(), cursor.sequence()).unwrap();
    assert_eq!(reset.position_after(cursor), CursorPosition::ResetRequired);
    let mut target = cursor.target();
    target.execution_id = ExecutionId::new();
    let replaced = ObservationCursor::new(target, cursor.stream(), cursor.sequence()).unwrap();
    assert_eq!(
        replaced.position_after(cursor),
        CursorPosition::ResetRequired
    );
    let maximum = at(u64::MAX);
    assert_eq!(maximum.position_after(maximum), CursorPosition::Duplicate);
    assert_eq!(
        at(u64::MAX - SEQUENCE_STEP).position_after(maximum),
        CursorPosition::Older
    );
    assert_eq!(
        maximum.position_after(at(u64::MAX - SEQUENCE_STEP)),
        CursorPosition::Next
    );
}
