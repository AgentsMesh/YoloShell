use crate::{constants::*, framing};
use tokio::{io::AsyncWriteExt, time::Instant};
use yolo_tui_client_control_protocol::ControlResponse;

const TEST_BUFFER_BYTES: usize = 8192;

#[tokio::test]
async fn frame_rejects_empty_oversized_and_truncated_messages() {
    for bytes in [
        u32::MIN.to_be_bytes().to_vec(),
        u32::MAX.to_be_bytes().to_vec(),
        vec![1],
        [
            u32::try_from(b"{}".len()).unwrap().to_be_bytes().as_slice(),
            b"{}",
        ]
        .concat(),
    ] {
        let (mut writer, mut reader) = tokio::io::duplex(TEST_BUFFER_BYTES);
        writer.write_all(&bytes).await.unwrap();
        drop(writer);
        assert!(
            framing::receive::<_, ControlResponse>(
                &mut reader,
                Instant::now() + CONTROL_REQUEST_TIMEOUT
            )
            .await
            .is_err()
        );
    }
}

#[tokio::test]
async fn frame_round_trip_and_expired_deadline_never_wait_indefinitely() {
    let (mut writer, mut reader) = tokio::io::duplex(TEST_BUFFER_BYTES);
    let deadline = Instant::now() + CONTROL_REQUEST_TIMEOUT;
    framing::send(&mut writer, &ControlResponse::accepted(), deadline)
        .await
        .unwrap();
    assert!(
        framing::receive::<_, ControlResponse>(&mut reader, deadline)
            .await
            .unwrap()
            .confirms_detached()
    );
    let expired = Instant::now() - CONTROL_REQUEST_TIMEOUT;
    assert!(
        framing::receive::<_, ControlResponse>(&mut reader, expired)
            .await
            .is_err()
    );
}
