use crate::fixtures::*;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    ReadTerminalHistoryFields, ReadTerminalHistoryRequest, ReadTerminalTextFields,
    ReadTerminalTextRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalHistoryRead,
    TerminalHistoryReadFields, TerminalSize, TerminalText, TerminalTextFields,
};

const SAMPLE_TEXT_SEQUENCE: u64 = 1;

pub fn samples() -> Vec<(Command, Reply)> {
    vec![
        (
            Command::ReadTerminalText {
                request: ReadTerminalTextRequest::new(ReadTerminalTextFields {
                    schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
                    target: target(),
                })
                .unwrap(),
            },
            Reply::TerminalText {
                text: TerminalText::new(TerminalTextFields {
                    target: target(),
                    sequence: SAMPLE_TEXT_SEQUENCE,
                    size: TerminalSize::default(),
                    text: "screen".into(),
                    truncated: false,
                })
                .unwrap(),
            },
        ),
        (
            Command::ReadTerminalHistory {
                request: ReadTerminalHistoryRequest::new(ReadTerminalHistoryFields {
                    schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
                    target: target(),
                    max_bytes: None,
                })
                .unwrap(),
            },
            Reply::TerminalHistory {
                history: TerminalHistoryRead::new(TerminalHistoryReadFields {
                    target: target(),
                    data: String::new(),
                    from_sequence: SAMPLE_TEXT_SEQUENCE,
                    next_sequence: SAMPLE_TEXT_SEQUENCE,
                    truncated: false,
                })
                .unwrap(),
            },
        ),
    ]
}
