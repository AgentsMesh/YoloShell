use super::{constants::*, fixture};
use crate::command_io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::watch,
    time::timeout,
};
use yolo_hub_relay_control_protocol::encode_request;

#[tokio::test]
async fn parent_cancellation_retires_idle_partial_and_queued_commands_without_waiting_for_input() {
    for stage in [Stage::Idle, Stage::Partial, Stage::Queued] {
        let request = fixture::request();
        let (io, mut incoming, mut peer) = fixture::channel(&request);
        let (cancel, cancelled) = watch::channel(false);
        let task = tokio::spawn(command_io::run(Some(io), cancel.clone(), cancelled));
        let queued = match stage {
            Stage::Idle => None,
            Stage::Partial => {
                peer.write_all(TRUNCATED_FRAME).await.unwrap();
                None
            }
            Stage::Queued => {
                let mut bytes = encode_request(&request).unwrap();
                let mut framed = (bytes.len() as u32).to_be_bytes().to_vec();
                framed.append(&mut bytes);
                bytes = framed;
                peer.write_all(&bytes).await.unwrap();
                Some(
                    timeout(TEST_TIMEOUT, incoming.incoming.recv())
                        .await
                        .unwrap()
                        .unwrap(),
                )
            }
        };
        cancel.send(true).unwrap();
        timeout(TEST_TIMEOUT, task).await.unwrap().unwrap().unwrap();
        if let Some(queued) = queued {
            assert!(queued.response.is_closed());
        }
        assert!(incoming.incoming.recv().await.is_none());
        let mut byte = [u8::default()];
        match timeout(TEST_TIMEOUT, peer.read(&mut byte)).await.unwrap() {
            Ok(count) => assert_eq!(count, usize::default()),
            // Cancellation can close the socket before consuming partial input.
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset),
        }
    }
}

enum Stage {
    Idle,
    Partial,
    Queued,
}

#[tokio::test]
async fn legacy_no_command_channel_and_already_cancelled_start_finish_without_peer_activity() {
    for initially_stopped in [false, true] {
        let (cancel, cancelled) = watch::channel(initially_stopped);
        let task = tokio::spawn(command_io::run(None, cancel.clone(), cancelled));
        if !initially_stopped {
            cancel.send(true).unwrap();
        }
        timeout(TEST_TIMEOUT, task).await.unwrap().unwrap().unwrap();
    }
}

#[tokio::test]
async fn malformed_frame_broadcasts_shutdown_and_never_reaches_relay_queue() {
    let (io, mut incoming, mut peer) = fixture::channel(&fixture::request());
    let (cancel, cancelled) = watch::channel(false);
    let observed = cancelled.clone();
    let task = tokio::spawn(command_io::run(Some(io), cancel, cancelled));
    peer.write_all(INVALID_FRAME).await.unwrap();
    assert!(timeout(TEST_TIMEOUT, task).await.unwrap().unwrap().is_err());
    assert!(*observed.borrow());
    assert!(incoming.incoming.recv().await.is_none());
}
