use serde::{Serialize, Serializer};
use tokio::{
    io::duplex,
    time::{Duration, Instant, timeout},
};
use yolo_hub_jsonrpc_transport::*;

const TIMEOUT: Duration = Duration::from_secs(1);
const PIPE_BYTES: usize = 64;
struct Failure;
impl Serialize for Failure {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("injected encoding failure"))
    }
}

#[tokio::test(start_paused = true)]
async fn preflight_failure_preserves_the_channel_but_inflight_cancellation_closes_it() {
    let (left, _right) = duplex(PIPE_BYTES);
    let mut framed = Framed::new(left);
    assert!(matches!(
        framed.send(&Failure, Instant::now() + TIMEOUT).await,
        Err(TransportError::Encode(_))
    ));
    assert!(framed.is_usable());
    assert!(matches!(
        framed.send(&true, Instant::now()).await,
        Err(TransportError::Deadline)
    ));
    assert!(framed.is_usable());
    assert!(matches!(
        framed.receive::<bool>(Instant::now()).await,
        Err(TransportError::Deadline)
    ));
    assert!(framed.is_usable());
    assert!(
        timeout(
            TIMEOUT,
            framed.receive::<bool>(Instant::now() + TIMEOUT + TIMEOUT)
        )
        .await
        .is_err()
    );
    assert!(!framed.is_usable());
    assert!(matches!(
        framed.close(Instant::now() + TIMEOUT).await,
        Err(TransportError::Poisoned)
    ));
}

#[tokio::test(start_paused = true)]
async fn write_timeout_read_timeout_eof_and_broken_pipe_are_terminal() {
    let (left, _right) = duplex(1);
    let mut framed = Framed::new(left);
    assert!(matches!(
        framed.send(&true, Instant::now() + TIMEOUT).await,
        Err(TransportError::Deadline)
    ));
    assert!(!framed.is_usable());
    let (left, _right) = duplex(PIPE_BYTES);
    let mut framed = Framed::new(left);
    assert!(matches!(
        framed.receive::<bool>(Instant::now() + TIMEOUT).await,
        Err(TransportError::Deadline)
    ));
    assert!(!framed.is_usable());
    let (left, right) = duplex(PIPE_BYTES);
    drop(right);
    let mut framed = Framed::new(left);
    assert!(matches!(
        framed.send(&true, Instant::now() + TIMEOUT).await,
        Err(TransportError::Io(_))
    ));
    assert!(!framed.is_usable());
    let (left, right) = duplex(PIPE_BYTES);
    drop(right);
    let mut framed = Framed::new(left);
    assert_eq!(
        framed
            .receive::<bool>(Instant::now() + TIMEOUT)
            .await
            .unwrap(),
        Incoming::Eof
    );
    assert!(!framed.is_usable());
}

#[tokio::test]
async fn graceful_close_signals_eof_and_retires_both_ends() {
    let (left, right) = duplex(PIPE_BYTES);
    let mut sender = Framed::new(left);
    let mut receiver = Framed::new(right);
    sender.close(Instant::now() + TIMEOUT).await.unwrap();
    assert!(!sender.is_usable());
    assert_eq!(
        receiver
            .receive::<bool>(Instant::now() + TIMEOUT)
            .await
            .unwrap(),
        Incoming::Eof
    );
}
