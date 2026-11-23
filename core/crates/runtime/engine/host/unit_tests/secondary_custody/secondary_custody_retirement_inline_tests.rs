use super::*;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn succession_ordinal_exhaustion_fails_closed_without_wrapping() {
    let mut succession = SecondaryCustodySuccession { latest: u64::MAX };

    let error = succession.allocate().unwrap_err();

    assert!(matches!(
        error,
        WorkerRpcError::Protocol(detail) if detail == SUCCESSION_ORDINAL_EXHAUSTED_DETAIL
    ));
    assert_eq!(succession.latest, u64::MAX);
}
