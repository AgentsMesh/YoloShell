use super::*;

#[test]
fn only_explicit_close_is_an_exact_terminal_event() {
    assert!(matches!(
        receive(Some(ChannelMsg::Close)),
        Received::PeerClose
    ));
}

#[test]
fn confirmed_open_failure_is_fail_closed() {
    let message = ChannelMsg::OpenFailure(russh::ChannelOpenFailure::AdministrativelyProhibited);
    assert_recovery(receive(Some(message)), SFTP_CHANNEL_OPEN_FAILURE_MESSAGE);
}

#[test]
fn unexplained_receiver_eof_is_fail_closed() {
    assert_recovery(receive(None), SFTP_CHANNEL_RECEIVER_EOF_MESSAGE);
}

fn assert_recovery(received: Received, expected: &str) {
    let Received::Failed(error) = received else {
        unreachable!()
    };
    assert!(matches!(&error, RemoteError::Transport(message) if message == expected));
    assert!(error.requires_connection_recovery());
}
