use super::fixtures;
use base64::Engine;
use yolo_hub_terminal_protocol::{
    DEFAULT_TERMINAL_HISTORY_BYTES, MAX_TERMINAL_HISTORY_BYTES, ReadTerminalHistoryFields,
    ReadTerminalHistoryRequest, TerminalHistoryRead, TerminalHistoryReadFields,
    TERMINAL_PROTOCOL_SCHEMA_VERSION,
};

fn request(max_bytes: Option<u32>) -> ReadTerminalHistoryRequest {
    ReadTerminalHistoryRequest::new(ReadTerminalHistoryFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        target: fixtures::target(),
        max_bytes,
    })
    .unwrap()
}

fn result(data: &[u8]) -> TerminalHistoryReadFields {
    TerminalHistoryReadFields {
        target: fixtures::target(),
        data: base64::engine::general_purpose::STANDARD.encode(data),
        from_sequence: 1,
        next_sequence: 3,
        truncated: false,
    }
}

#[test]
fn history_request_defaults_and_bounds_are_validated() {
    assert_eq!(request(None).max_bytes(), DEFAULT_TERMINAL_HISTORY_BYTES);
    assert_eq!(request(Some(1024)).max_bytes(), 1024);
    for value in [0, MAX_TERMINAL_HISTORY_BYTES as u32 + 1] {
        assert!(request_with(Some(value)).is_err());
    }
    assert!(
        ReadTerminalHistoryRequest::new(ReadTerminalHistoryFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION + 1,
            target: fixtures::target(),
            max_bytes: None,
        })
        .is_err()
    );
}

fn request_with(max_bytes: Option<u32>) -> Result<ReadTerminalHistoryRequest, yolo_hub_terminal_protocol::PublicError> {
    ReadTerminalHistoryRequest::new(ReadTerminalHistoryFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        target: fixtures::target(),
        max_bytes,
    })
}

#[test]
fn history_result_decodes_the_bounded_raw_stream() {
    let bytes = TerminalHistoryRead::new(result(b"hello\x1b[0m")).unwrap();
    assert_eq!(bytes.bytes().unwrap(), b"hello\x1b[0m");
    let mut fields = result(b"ok");
    fields.data = "not base64!".into();
    assert!(TerminalHistoryRead::new(fields).is_err());
    let mut fields = result(b"ok");
    fields.from_sequence = 4;
    assert!(TerminalHistoryRead::new(fields).is_err());
    let mut fields = result(b"ok");
    fields.next_sequence = 0;
    assert!(TerminalHistoryRead::new(fields).is_err());
}
