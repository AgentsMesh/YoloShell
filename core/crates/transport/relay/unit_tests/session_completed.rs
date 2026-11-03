use super::*;
use crate::protocol::method;
use crate::relay::constants::TERMINAL_RESET;
use crate::relay::session_test_support::{
    completed_session, exit_event, host_status, options, snapshot_attach,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, duplex};
use tokio::sync::mpsc;

const CAPTURE_BYTES: usize = 128;
const IGNORED_INPUT: &[u8] = b"must not reach a completed runtime";
const FINAL_SCREEN: &[u8] = b"completed terminal screen";
const RESIZE_COLUMNS: u16 = 101;
const RESIZE_ROWS: u16 = 41;
const RESIZE_CHANNEL_CAPACITY: usize = 1;

#[tokio::test]
async fn completed_runtime_replays_output_without_control_or_resume() {
    let terminal_id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
        Reply::attachment(snapshot_attach(FINAL_SCREEN), vec![exit_event(terminal_id)]),
    ])
    .await;
    let (resize_sender, resize_events) = mpsc::channel(RESIZE_CHANNEL_CAPACITY);
    resize_sender
        .send(TerminalSize::new(RESIZE_COLUMNS, RESIZE_ROWS).unwrap())
        .await
        .unwrap();
    drop(resize_sender);
    let (output, mut captured) = duplex(CAPTURE_BYTES);
    let (status, mut statuses) = mpsc::unbounded_channel();

    let exit = relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id).take_control(true),
        Cursor::new(IGNORED_INPUT),
        output,
        resize_events,
        status,
    )
    .await
    .unwrap();

    assert_eq!(exit, RelayExit::SessionExited);
    let mut bytes = Vec::new();
    captured.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes, [TERMINAL_RESET, FINAL_SCREEN].concat());
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionEnding);
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionExited);
    assert!(statuses.try_recv().is_err());
    assert_eq!(
        host.methods(),
        [
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ATTACH_SESSION,
        ]
    );
}
