use super::*;

const INITIAL_INPUT: &[u8] = b"pinned input";
const APPENDED_INPUT: &[u8] = b" later input";

fn result(input_id: Uuid, delivery: &str) -> InputResult {
    InputResult {
        input_id,
        duplicate: false,
        delivery: delivery.into(),
    }
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn retry_pins_id_and_payload_until_confirmation() {
    let mut pending = PendingInputState::new();
    pending.append(INITIAL_INPUT);
    let first = pending.next_delivery().unwrap();

    pending.append(APPENDED_INPUT);
    assert_eq!(pending.next_delivery(), Some(first.clone()));
    pending
        .confirm(&first, &result(first.input_id, delivery::DELIVERED))
        .unwrap();

    let next = pending.next_delivery().unwrap();
    assert_ne!(next.input_id, first.input_id);
    assert_eq!(next.data, APPENDED_INPUT);
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn invalid_result_preserves_the_exact_pending_delivery() {
    let mut pending = PendingInputState::new();
    pending.append(INITIAL_INPUT);
    let expected = pending.next_delivery().unwrap();
    let invalid = [
        result(Uuid::nil(), delivery::DELIVERED),
        result(expected.input_id, delivery::UNSUPPORTED),
    ];

    for result in invalid {
        assert!(matches!(
            pending.confirm(&expected, &result),
            Err(RelayError::Protocol(ProtocolError::ResponseMismatch))
        ));
        assert_eq!(pending.next_delivery(), Some(expected.clone()));
    }
}
