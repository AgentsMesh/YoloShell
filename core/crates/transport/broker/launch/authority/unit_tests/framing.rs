use crate::{constants::*, frame, wire::Request};
use std::io::Write;
use yolo_local_unix::BlockingLocalUnixStream;

const ACCEPTED_FIELD: &str = "accepted";
const STALL_DEADLINE: std::time::Duration = std::time::Duration::from_millis(40);
const SCHEDULING_MARGIN: std::time::Duration = std::time::Duration::from_secs(1);

fn complete_value() -> Vec<u8> {
    yolo_capnp_codec::encode(&serde_json::json!({ACCEPTED_FIELD:true})).unwrap()
}

#[test]
fn oversized_frame_is_rejected_before_allocating_or_reading_payload() {
    let (mut sender, mut receiver) = BlockingLocalUnixStream::pair().unwrap();
    sender
        .write_all(
            &u32::try_from(MAX_FRAME_BYTES + COUNTER_STEP as usize)
                .unwrap()
                .to_be_bytes(),
        )
        .unwrap();
    assert!(
        frame::read::<Request>(
            &mut receiver,
            tokio::time::Instant::now() + HANDSHAKE_TIMEOUT
        )
        .is_err()
    );
}

#[test]
fn eof_and_expired_deadline_cannot_produce_a_partial_grant() {
    let (sender, mut receiver) = BlockingLocalUnixStream::pair().unwrap();
    drop(sender);
    assert!(
        frame::read::<Request>(
            &mut receiver,
            tokio::time::Instant::now() + HANDSHAKE_TIMEOUT
        )
        .is_err()
    );
    let (_sender, mut receiver) = BlockingLocalUnixStream::pair().unwrap();
    assert!(frame::read::<Request>(&mut receiver, tokio::time::Instant::now()).is_err());
}

#[test]
fn complete_buffered_frame_survives_peer_close_before_header_or_payload_read() {
    let complete_value = complete_value();
    let (mut sender, mut receiver) = BlockingLocalUnixStream::pair().unwrap();
    sender
        .write_all(&u32::try_from(complete_value.len()).unwrap().to_be_bytes())
        .unwrap();
    sender.write_all(&complete_value).unwrap();
    drop(sender);
    let received: serde_json::Value = frame::read(
        &mut receiver,
        tokio::time::Instant::now() + HANDSHAKE_TIMEOUT,
    )
    .unwrap();
    assert_eq!(
        received,
        serde_json::json!({ACCEPTED_FIELD:true})
    );
}

#[test]
fn peer_close_with_truncated_header_or_payload_never_accepts_an_ack() {
    let complete_value = complete_value();
    let header = u32::try_from(complete_value.len()).unwrap().to_be_bytes();
    for bytes in [
        &header[..header.len() - COUNTER_STEP as usize],
        header.as_slice(),
    ] {
        let (mut sender, mut receiver) = BlockingLocalUnixStream::pair().unwrap();
        sender.write_all(bytes).unwrap();
        drop(sender);
        assert!(
            frame::read::<serde_json::Value>(
                &mut receiver,
                tokio::time::Instant::now() + HANDSHAKE_TIMEOUT
            )
            .is_err()
        );
    }
    let (mut sender, mut receiver) = BlockingLocalUnixStream::pair().unwrap();
    sender.write_all(&header).unwrap();
    sender
        .write_all(&complete_value[..complete_value.len() - COUNTER_STEP as usize])
        .unwrap();
    drop(sender);
    assert!(
        frame::read::<serde_json::Value>(
            &mut receiver,
            tokio::time::Instant::now() + HANDSHAKE_TIMEOUT
        )
        .is_err()
    );
}

#[test]
fn stalled_frame_remains_bounded_by_the_original_absolute_deadline() {
    let complete_value = complete_value();
    let (mut sender, mut receiver) = BlockingLocalUnixStream::pair().unwrap();
    sender
        .write_all(&u32::try_from(complete_value.len()).unwrap().to_be_bytes())
        .unwrap();
    let deadline = tokio::time::Instant::now() + STALL_DEADLINE;
    let result = frame::read::<serde_json::Value>(&mut receiver, deadline);
    assert!(result.is_err());
    assert!(tokio::time::Instant::now() >= deadline);
    assert!(tokio::time::Instant::now() < deadline + SCHEDULING_MARGIN);
}
