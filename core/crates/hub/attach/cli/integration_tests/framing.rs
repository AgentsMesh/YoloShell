use super::{constants::*, fixture};
use crate::command_frames;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;
use yolo_hub_relay_control_protocol::{
    MAX_FRAME_BYTES, Outcome, Response, decode_response, encode_request,
};

#[tokio::test]
async fn split_frames_are_read_exactly_and_responses_echo_the_original_intent() {
    let request = fixture::request();
    let payload = encode_request(&request).unwrap();
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend(payload);
    let (mut input, mut output) = tokio::io::duplex(IO_CAPACITY);
    let producer = async {
        for byte in &frame {
            output.write_all(std::slice::from_ref(byte)).await.unwrap();
        }
    };
    let (parsed, ()) = timeout(TEST_TIMEOUT, async { tokio::join!(command_frames::read(&mut input), producer) }).await.unwrap();
    assert_eq!(parsed.unwrap(), request);
    let response = Response::new(&request, Outcome::Unknown {}).unwrap();
    command_frames::write(&mut input, &response).await.unwrap();
    let length = output.read_u32().await.unwrap() as usize;
    assert!(length <= MAX_FRAME_BYTES);
    let mut bytes = vec![0; length];
    output.read_exact(&mut bytes).await.unwrap();
    decode_response(&bytes)
        .unwrap()
        .validate_for(&request)
        .unwrap();
}

#[tokio::test]
async fn malformed_truncated_and_oversized_commands_fail_before_relay_admission() {
    let oversized = (MAX_FRAME_BYTES as u32 + 1).to_be_bytes();
    for invalid in [INVALID_FRAME, TRUNCATED_FRAME, oversized.as_slice()] {
        let (mut read, mut write) = tokio::io::duplex(IO_CAPACITY);
        write.write_all(invalid).await.unwrap();
        write.shutdown().await.unwrap();
        assert!(timeout(TEST_TIMEOUT, command_frames::read(&mut read)).await.unwrap().is_err());
    }
}

#[tokio::test]
async fn a_response_write_failure_never_reports_success() {
    let (mut stream, peer) = tokio::io::duplex(IO_CAPACITY);
    drop(peer);
    let response = Response::new(&fixture::request(), Outcome::Unknown {}).unwrap();
    assert!(command_frames::write(&mut stream, &response).await.is_err());
}
