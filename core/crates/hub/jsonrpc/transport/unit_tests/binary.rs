use tokio::{
    io::duplex,
    time::{Duration, Instant},
};
use yolo_hub_jsonrpc_transport::{Framed, TransportError};

const LIMIT: usize = 8;
const TIMEOUT: Duration = Duration::from_secs(1);

#[tokio::test(start_paused = true)]
async fn binary_preflight_and_transfer_timeouts_have_distinct_custody() {
    let (left, _right) = duplex(1);
    let mut sender = Framed::new(left);
    assert!(matches!(
        sender
            .send_binary(&[0; LIMIT + 1], LIMIT, Instant::now() + TIMEOUT)
            .await,
        Err(TransportError::FrameTooLarge)
    ));
    assert!(sender.is_usable());
    assert!(matches!(
        sender.send_binary(&[0], LIMIT, Instant::now()).await,
        Err(TransportError::Deadline)
    ));
    assert!(sender.is_usable());
    assert!(matches!(
        sender
            .send_binary(&[0], LIMIT, Instant::now() + TIMEOUT)
            .await,
        Err(TransportError::Deadline)
    ));
    assert!(!sender.is_usable());
    assert!(matches!(
        sender
            .send_binary(&[], LIMIT, Instant::now() + TIMEOUT)
            .await,
        Err(TransportError::Poisoned)
    ));
    let (left, _right) = duplex(LIMIT);
    let mut receiver = Framed::new(left);
    assert!(matches!(
        receiver.receive_binary(1, LIMIT, Instant::now()).await,
        Err(TransportError::Deadline)
    ));
    assert!(receiver.is_usable());
    assert!(matches!(
        receiver
            .receive_binary(1, LIMIT, Instant::now() + TIMEOUT)
            .await,
        Err(TransportError::Deadline)
    ));
    assert!(!receiver.is_usable());
}

#[tokio::test]
async fn empty_binary_snapshot_has_an_explicit_frame_and_io_failure_retires_writer() {
    let (left, right) = duplex(LIMIT);
    let mut sender = Framed::new(left);
    let mut receiver = Framed::new(right);
    sender
        .send_binary(&[], LIMIT, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    assert!(
        receiver
            .receive_binary(0, LIMIT, Instant::now() + TIMEOUT)
            .await
            .unwrap()
            .is_empty()
    );
    drop(receiver);
    assert!(matches!(
        sender
            .send_binary(&[0], LIMIT, Instant::now() + TIMEOUT)
            .await,
        Err(TransportError::Io(_))
    ));
    assert!(!sender.is_usable());
}
