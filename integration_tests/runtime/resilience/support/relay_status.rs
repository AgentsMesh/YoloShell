use tokio::io::{AsyncRead, AsyncReadExt};
use yolo_hub_relay_control_protocol::{MAX_STATUS_BYTES, Status, decode_status};

pub(crate) async fn read_relay_status(reader: &mut (impl AsyncRead + Unpin)) -> Status {
    let length = reader.read_u32().await.unwrap() as usize;
    assert!(length <= MAX_STATUS_BYTES);
    let mut bytes = vec![u8::default(); length];
    reader.read_exact(&mut bytes).await.unwrap();
    decode_status(&bytes).unwrap()
}
