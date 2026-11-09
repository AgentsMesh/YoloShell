use super::*;
use std::path::PathBuf;
use yolo_hub_relay_control_protocol::{decode_status, encode_status, Status, StatusKind};

const HOLDER: &str = "status-owner";

#[test]
fn every_relay_status_maps_to_a_typed_binary_message() {
    let cases = [
        (RelayStatus::ControlAcquired, StatusKind::ControlAcquired, None),
        (RelayStatus::ReadOnly { holder_id: Some(HOLDER.into()) }, StatusKind::ReadOnly, Some(HOLDER)),
        (RelayStatus::ControlLost { holder_id: None }, StatusKind::ControlLost, None),
        (RelayStatus::HostStarted { executable: PathBuf::from("/tmp/yolo") }, StatusKind::HostStarted, None),
        (RelayStatus::SessionResumed, StatusKind::SessionResumed, None),
        (RelayStatus::SessionEnding, StatusKind::SessionEnding, None),
        (RelayStatus::SessionExited, StatusKind::SessionExited, None),
    ];
    for (status, kind, holder) in cases {
        let expected = Status { kind, holder_id: holder.map(str::to_owned) };
        let native = StatusFrame::from(status).native();
        assert_eq!(native, expected);
        assert_eq!(decode_status(&encode_status(&native).unwrap()).unwrap(), expected);
    }
}

#[test]
fn non_ownership_holder_is_rejected_by_the_shared_native_codec() {
    for kind in [StatusKind::ControlAcquired, StatusKind::SessionEnding, StatusKind::SessionExited] {
        assert!(encode_status(&Status { kind, holder_id: Some(HOLDER.into()) }).is_err());
    }
    assert!(decode_status(br#"{"event":"control_acquired","holder_id":null}"#).is_err());
}
