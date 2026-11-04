use super::*;
use std::io::Cursor;

const ROOT_PATH: &str = "/private/tmp/yolo-guard-protocol";
const VALID_PID: u32 = 41;
const VALID_START_TIME: u64 = 73;
const VALID_DESCRIPTOR: u64 = 91;
const VALID_PEER_DESCRIPTOR: u64 = 92;
const UNKNOWN_FRAME: &str = "unknown";
const INVALID_SECRET: &str = "not-a-capability";
const LOWERCASE_SECRET_CHARACTER: &str = "a";
const UPPERCASE_SECRET_CHARACTER: &str = "A";
const TRUNCATED_CAPABILITY_HEX_LENGTH: usize = CAPABILITY_HEX_LENGTH - 1;

fn identity() -> ProcessIdentity {
    ProcessIdentity {
        pid: VALID_PID,
        start_time: VALID_START_TIME,
    }
}

#[test]
fn root_frame_keeps_control_and_discovery_capabilities_distinct() {
    let control = ControlCapability::generate();
    let discovery = DiscoveryCapability::generate();
    assert_ne!(control.value(), discovery.value());
    let mut encoded = Vec::new();
    write_root(
        &mut encoded,
        &control,
        &discovery,
        VALID_DESCRIPTOR,
        VALID_PEER_DESCRIPTOR,
        Path::new(ROOT_PATH),
    )
    .unwrap();
    let parsed = read_root(&mut Cursor::new(encoded)).unwrap();
    assert!(control.matches(parsed.capability.value()));
    assert_eq!(parsed.discovery.value(), discovery.value());
    assert_eq!(parsed.discovery_reader, VALID_DESCRIPTOR);
    assert_eq!(parsed.discovery_writer, VALID_PEER_DESCRIPTOR);
    assert_eq!(parsed.path, PathBuf::from(ROOT_PATH));
}

#[test]
fn authenticated_reader_helpers_decode_complete_frames() {
    let capability = ControlCapability::generate();
    let mut encoded = Vec::new();
    write_identity(&mut encoded, FRAME_TRACK, &capability, identity()).unwrap();
    assert_eq!(
        read_identity(&mut Cursor::new(encoded), FRAME_TRACK, &capability).unwrap(),
        identity()
    );

    let mut encoded = Vec::new();
    write_capability(&mut encoded, FRAME_FINISH, &capability).unwrap();
    read_capability(&mut Cursor::new(encoded.clone()), FRAME_FINISH, &capability).unwrap();
    let parsed = read_new_capability(&mut Cursor::new(encoded), FRAME_FINISH).unwrap();
    assert!(capability.matches(parsed.value()));
}

#[test]
fn descriptor_frames_bind_the_tracked_root() {
    let capability = ControlCapability::generate();
    let mut encoded = Vec::new();
    write_tracked_root(
        &mut encoded,
        &capability,
        identity(),
        VALID_DESCRIPTOR,
        identity(),
    )
    .unwrap();
    assert_eq!(
        read_tracked_root(&mut Cursor::new(encoded), &capability).unwrap(),
        (identity(), VALID_DESCRIPTOR, identity())
    );
}

#[test]
fn descriptor_frames_reject_missing_authority() {
    let capability = ControlCapability::generate();
    assert!(
        write_tracked_root(
            &mut Vec::new(),
            &capability,
            identity(),
            u64::default(),
            identity()
        )
        .is_err()
    );
}

#[test]
fn frame_writer_rejects_embedded_protocol_delimiters() {
    for invalid in [FRAME_SEPARATOR.to_string(), FRAME_TERMINATOR.to_string()] {
        assert!(frame::write_fields(&mut Vec::new(), &[&invalid]).is_err());
    }
}

#[test]
fn capability_comparison_rejects_every_mismatch_shape() {
    let expected = ControlCapability::generate();
    let different = ControlCapability::generate();
    assert!(expected.matches(expected.value()));
    assert!(!expected.matches(different.value()));
    assert!(!expected.matches(INVALID_SECRET));
    assert!(!expected.matches(""));
}

#[test]
fn identity_frames_require_capability_kind_and_nonzero_generation() {
    let capability = ControlCapability::generate();
    let mut encoded = Vec::new();
    write_identity(&mut encoded, FRAME_TRACK, &capability, identity()).unwrap();
    let encoded = String::from_utf8(encoded).unwrap();
    assert_eq!(
        parse_identity(&encoded, FRAME_TRACK, &capability).unwrap(),
        identity()
    );

    let other = ControlCapability::generate();
    assert!(parse_identity(&encoded, FRAME_TRACK, &other).is_err());
    assert!(parse_identity(&encoded, FRAME_TRACKED, &capability).is_err());
    for invalid in [
        ProcessIdentity {
            pid: u32::default(),
            start_time: VALID_START_TIME,
        },
        ProcessIdentity {
            pid: VALID_PID,
            start_time: u64::default(),
        },
    ] {
        let mut encoded = Vec::new();
        write_identity(&mut encoded, FRAME_TRACK, &capability, invalid).unwrap();
        let encoded = String::from_utf8(encoded).unwrap();
        assert!(parse_identity(&encoded, FRAME_TRACK, &capability).is_err());
    }
}

#[test]
fn malformed_and_oversized_frames_fail_closed() {
    let capability = ControlCapability::generate();
    for invalid in [
        String::new(),
        format!("{UNKNOWN_FRAME}{FRAME_TERMINATOR}"),
        format!("{FRAME_FINISH}{FRAME_SEPARATOR}{INVALID_SECRET}"),
        "x".repeat(
            usize::try_from(MAXIMUM_CONTROL_FRAME_BYTES + CONTROL_FRAME_OVERFLOW_SENTINEL_BYTES)
                .unwrap(),
        ),
    ] {
        assert!(parse_capability(&invalid, FRAME_FINISH, &capability).is_err());
    }
}

#[test]
fn capability_parser_rejects_truncated_uppercase_and_non_hex_values() {
    let valid = LOWERCASE_SECRET_CHARACTER.repeat(CAPABILITY_HEX_LENGTH);
    let uppercase = UPPERCASE_SECRET_CHARACTER.repeat(CAPABILITY_HEX_LENGTH);
    let truncated = LOWERCASE_SECRET_CHARACTER.repeat(TRUNCATED_CAPABILITY_HEX_LENGTH);
    assert!(ControlCapability::parse(&valid).is_ok());
    assert!(ControlCapability::parse(INVALID_SECRET).is_err());
    assert!(ControlCapability::parse(&uppercase).is_err());
    assert!(ControlCapability::parse(&truncated).is_err());
}
