use super::*;

const OTHER_UID_OFFSET: libc::uid_t = 1;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn peer_uid_validation_rejects_a_different_effective_user() {
    let expected = libc::uid_t::default();
    validate_peer_uid(expected, expected).unwrap();
    let error = validate_peer_uid(expected + OTHER_UID_OFFSET, expected).unwrap_err();
    assert!(matches!(
        error,
        ClientError::Protocol(ProtocolError::Io(ref source))
            if source.kind() == std::io::ErrorKind::PermissionDenied
    ));
}
