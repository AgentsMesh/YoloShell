mod bootstrap_dispatch;
use crate::bootstrap::capture;
use yolo_hub_local_startup::MAX_READY_BYTES;

#[tokio::test]
async fn receipt_capture_preserves_exact_bytes_and_rejects_overflow() {
    let exact = vec![b'x'; MAX_READY_BYTES];
    assert_eq!(capture(exact.as_slice()).await.unwrap(), exact);
    let overflow = vec![b'x'; MAX_READY_BYTES + crate::constants::OVERFLOW_BYTE as usize];
    assert!(capture(overflow.as_slice()).await.is_err());
    assert!(capture(&b""[..]).await.unwrap().is_empty());
}
