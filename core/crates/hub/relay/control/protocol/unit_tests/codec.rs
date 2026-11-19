use crate::fixtures::*;
use yolo_hub_relay_control_protocol::*;

#[test]
fn typed_capnp_messages_roundtrip_and_reject_cross_kind_decoding() {
    let request = request();
    let encoded = encode_request(&request).unwrap();
    assert_eq!(decode_request(&encoded), Ok(request.clone()));
    assert_eq!(decode_response(&encoded), Err(ProtocolError::InvalidFrame));
    let response = response(&request);
    let encoded = encode_response(&response).unwrap();
    assert_eq!(decode_response(&encoded), Ok(response));
    assert_eq!(decode_request(&encoded), Err(ProtocolError::InvalidFrame));
}
#[test]
fn payload_limit_and_trailing_data_are_rejected_before_unbounded_traversal() {
    let request = encode_request(&request()).unwrap();
    let mut bytes = request.clone();
    bytes.extend_from_slice(&request);
    assert_eq!(decode_request(&bytes), Err(ProtocolError::InvalidFrame));
    bytes.resize(MAX_FRAME_BYTES + 1, 0);
    assert_eq!(decode_request(&bytes), Err(ProtocolError::FrameTooLarge));
    assert_eq!(decode_response(&bytes), Err(ProtocolError::FrameTooLarge));
}
#[test]
fn empty_truncated_and_retired_json_frames_are_rejected() {
    assert_eq!(decode_request(&[]), Err(ProtocolError::EmptyFrame));
    assert_eq!(decode_response(&[]), Err(ProtocolError::EmptyFrame));
    let request = encode_request(&request()).unwrap();
    for length in 0..request.len() {
        assert!(decode_request(&request[..length]).is_err());
    }
    for bytes in [b"{}".as_slice(), b"null", b"{\"schema_version\":2}\n", &[0xff; 16]] {
        assert_eq!(decode_request(bytes), Err(ProtocolError::InvalidFrame));
        assert_eq!(decode_response(bytes), Err(ProtocolError::InvalidFrame));
    }
}
#[test]
fn all_status_variants_preserve_only_the_allowed_holder() {
    for kind in [StatusKind::ControlAcquired, StatusKind::ReadOnly, StatusKind::ControlLost,
        StatusKind::HostStarted, StatusKind::SessionResumed, StatusKind::SessionEnding, StatusKind::SessionExited] {
        let status = Status { kind, holder_id: None };
        assert_eq!(decode_status(&encode_status(&status).unwrap()), Ok(status));
        let status = Status { kind, holder_id: Some("holder".into()) };
        if matches!(kind, StatusKind::ReadOnly | StatusKind::ControlLost) {
            assert_eq!(decode_status(&encode_status(&status).unwrap()), Ok(status));
        } else { assert!(encode_status(&status).is_err()); }
    }
    for holder in [String::new(), "x".repeat(MAX_HOLDER_BYTES + 1)] {
        assert!(encode_status(&Status { kind: StatusKind::ReadOnly, holder_id: Some(holder) }).is_err());
    }
}
