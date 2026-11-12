use super::fixture::TEST_DEADLINE;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    time::{Instant, timeout_at},
};
use yolo_tui_client_control_protocol::{ControlResponse, MAX_CONTROL_FRAME_BYTES};

pub async fn raw(socket: &std::path::Path, bytes: &[u8]) -> ControlResponse {
    let deadline = Instant::now() + TEST_DEADLINE;
    timeout_at(deadline, async {
        let mut stream = UnixStream::connect(socket).await.unwrap();
        stream.write_all(bytes).await.unwrap();
        stream.shutdown().await.unwrap();
        let mut header = [u8::default(); std::mem::size_of::<u32>()];
        stream.read_exact(&mut header).await.unwrap();
        let length = u32::from_be_bytes(header) as usize;
        assert!(length <= MAX_CONTROL_FRAME_BYTES);
        let mut payload = vec![u8::default(); length];
        stream.read_exact(&mut payload).await.unwrap();
        yolo_capnp_codec::decode(&payload).unwrap()
    })
    .await
    .unwrap()
}

pub fn frame(value: &impl serde::Serialize) -> Vec<u8> {
    let bytes = yolo_capnp_codec::encode(value).unwrap();
    let mut framed = u32::try_from(bytes.len()).unwrap().to_be_bytes().to_vec();
    framed.extend(bytes);
    framed
}
