use super::*;
use crate::remote::constants::{SSH_EXTENDED_DATA_STDERR, SUCCESS_EXIT_STATUS};

const TEST_OUTPUT_LIMIT: usize = 64;
const PRE_EOF_OUTPUT: &[u8] = b"accepted";
const POST_EOF_OUTPUT: &[u8] = b"rejected";

fn collector() -> CommandCollector {
    CommandCollector::new(TEST_OUTPUT_LIMIT)
}

#[test]
fn eof_rejects_every_later_output_kind() {
    for message in [
        ChannelMsg::Data {
            data: POST_EOF_OUTPUT.to_vec().into(),
        },
        ChannelMsg::ExtendedData {
            data: POST_EOF_OUTPUT.to_vec().into(),
            ext: SSH_EXTENDED_DATA_STDERR,
        },
    ] {
        let mut collector = collector();
        assert!(collector.accept(ChannelMsg::Eof).unwrap());
        assert!(matches!(
            collector.accept(message),
            Err(RemoteError::Transport(text)) if text == COMMAND_DATA_AFTER_EOF_MESSAGE
        ));
    }
}

#[test]
fn eof_keeps_exit_status_until_close_terminates_collection() {
    let mut collector = collector();
    collector
        .accept(ChannelMsg::Data {
            data: PRE_EOF_OUTPUT.to_vec().into(),
        })
        .unwrap();
    assert!(collector.accept(ChannelMsg::Eof).unwrap());
    assert!(
        collector
            .accept(ChannelMsg::ExitStatus {
                exit_status: SUCCESS_EXIT_STATUS,
            })
            .unwrap()
    );
    assert!(!collector.accept(ChannelMsg::Close).unwrap());
    let output = collector.finish();
    assert_eq!(output.stdout, PRE_EOF_OUTPUT);
    assert_eq!(output.exit_status, Some(SUCCESS_EXIT_STATUS));
}

#[test]
fn timeout_and_output_limit_states_are_preserved() {
    let mut collector = CommandCollector::new(PRE_EOF_OUTPUT.len());
    collector
        .accept(ChannelMsg::Data {
            data: [PRE_EOF_OUTPUT, POST_EOF_OUTPUT].concat().into(),
        })
        .unwrap();
    let output = collector.timed_out();
    assert_eq!(output.stdout, PRE_EOF_OUTPUT);
    assert!(output.truncated);
    assert!(output.timed_out);
    assert!(timed_out().timed_out);
}
