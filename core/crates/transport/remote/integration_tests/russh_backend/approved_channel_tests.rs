use super::*;
use crate::remote::constants::{SSH_EXTENDED_DATA_STDERR, SUCCESS_EXIT_STATUS};

const REPLAY_BYTES: &[u8] = b"prefix";
const POST_TERMINAL_BYTES: &[u8] = b"must-not-replay";
const PARTIAL_READ_BYTES: usize = 2;
const SINGLE_BUFFER_BYTE: usize = 1;
const INITIAL_REPLAY_OFFSET: usize = 0;
const CUSTOM_SIGNAL: &str = "YOLO_TEST_SIGNAL";
const SIGNAL_ERROR: &str = "remote signal failure";
const SIGNAL_LANGUAGE: &str = "en-US";
const SIGNAL_FILL: &str = "x";

#[test]
fn approval_buffer_enforces_bytes_messages_and_integer_overflow() {
    let mut oversized = ApprovalBuffer::new();
    let message = ChannelMsg::Data {
        data: vec![0; MAX_CHANNEL_APPROVAL_BYTES + SINGLE_BUFFER_BYTE].into(),
    };
    assert!(matches!(
        oversized.push(message),
        Err(RemoteError::LimitExceeded)
    ));

    let mut messages = ApprovalBuffer::new();
    for _ in 0..MAX_CHANNEL_APPROVAL_MESSAGES {
        messages.push(ChannelMsg::Eof).unwrap();
    }
    assert!(matches!(
        messages.push(ChannelMsg::Eof),
        Err(RemoteError::LimitExceeded)
    ));

    let mut overflow = ApprovalBuffer {
        messages: VecDeque::new(),
        bytes: usize::MAX,
    };
    assert!(matches!(
        overflow.push(ChannelMsg::Data {
            data: vec![0; SINGLE_BUFFER_BYTE].into(),
        }),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn approval_bytes_include_exit_signal_payload_and_fail_closed_on_overflow() {
    let mut measured = ApprovalBuffer::new();
    measured
        .push(ChannelMsg::ExitSignal {
            signal_name: russh::Sig::Custom(CUSTOM_SIGNAL.into()),
            core_dumped: false,
            error_message: SIGNAL_ERROR.into(),
            lang_tag: SIGNAL_LANGUAGE.into(),
        })
        .unwrap();
    assert_eq!(
        measured.bytes,
        CUSTOM_SIGNAL.len() + SIGNAL_ERROR.len() + SIGNAL_LANGUAGE.len()
    );

    let mut oversized = ApprovalBuffer::new();
    assert!(matches!(
        oversized.push(ChannelMsg::ExitSignal {
            signal_name: russh::Sig::TERM,
            core_dumped: false,
            error_message: SIGNAL_FILL.repeat(MAX_CHANNEL_APPROVAL_BYTES + SINGLE_BUFFER_BYTE),
            lang_tag: String::new(),
        }),
        Err(RemoteError::LimitExceeded)
    ));
    assert!(matches!(
        checked_payload_sum([usize::MAX, SINGLE_BUFFER_BYTE]),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn replay_keeps_standard_data_and_terminal_end_markers() {
    let messages = VecDeque::from([
        ChannelMsg::ExtendedData {
            ext: SSH_EXTENDED_DATA_STDERR,
            data: REPLAY_BYTES.to_vec().into(),
        },
        ChannelMsg::Data {
            data: REPLAY_BYTES.to_vec().into(),
        },
        ChannelMsg::ExitStatus {
            exit_status: SUCCESS_EXIT_STATUS,
        },
        ChannelMsg::Eof,
        ChannelMsg::Close,
    ]);

    let (prefix, eof) = replay_messages(messages);

    assert_eq!(
        prefix.into_iter().collect::<Vec<_>>(),
        vec![REPLAY_BYTES.to_vec()]
    );
    assert!(eof);
}

#[test]
fn replay_never_accepts_data_after_eof_or_close() {
    for terminal in [ChannelMsg::Eof, ChannelMsg::Close] {
        let messages = VecDeque::from([
            ChannelMsg::Data {
                data: REPLAY_BYTES.to_vec().into(),
            },
            terminal,
            ChannelMsg::Data {
                data: POST_TERMINAL_BYTES.to_vec().into(),
            },
        ]);

        let (prefix, eof) = replay_messages(messages);

        assert_eq!(prefix.into_iter().collect::<Vec<_>>(), [REPLAY_BYTES]);
        assert!(eof);
    }
}

#[test]
fn replay_prefix_skips_empty_chunks_and_preserves_partial_offsets() {
    let mut prefix = VecDeque::from([Vec::new(), REPLAY_BYTES.to_vec()]);
    let mut offset = INITIAL_REPLAY_OFFSET;
    let mut first = [0_u8; PARTIAL_READ_BYTES];
    let mut first_target = ReadBuf::new(&mut first);

    assert!(replay_prefix(&mut prefix, &mut offset, &mut first_target));
    assert_eq!(first_target.filled(), &REPLAY_BYTES[..PARTIAL_READ_BYTES]);
    assert_eq!(offset, PARTIAL_READ_BYTES);

    let mut remaining = [0_u8; REPLAY_BYTES.len()];
    let mut remaining_target = ReadBuf::new(&mut remaining);
    assert!(replay_prefix(
        &mut prefix,
        &mut offset,
        &mut remaining_target,
    ));
    assert_eq!(
        remaining_target.filled(),
        &REPLAY_BYTES[PARTIAL_READ_BYTES..]
    );
    assert_eq!(offset, INITIAL_REPLAY_OFFSET);
    assert!(!replay_prefix(
        &mut prefix,
        &mut offset,
        &mut remaining_target,
    ));
}
