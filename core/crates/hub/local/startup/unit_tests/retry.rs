use crate::{constants::*, retry};
use tokio::time::Instant;
use yolo_client::ClientError;
use yolo_wire::ProtocolError;

#[tokio::test]
async fn startup_probe_and_pause_remain_within_the_overall_budget() {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    assert!(retry::require_budget(deadline).is_ok());
    let before = Instant::now();
    let probe = retry::probe_deadline(deadline);
    assert!(probe >= before && probe <= Instant::now() + PROBE_TIMEOUT);
    assert!(probe < deadline);
    retry::pause(deadline).await.unwrap();
    assert!(Instant::now() >= before + POLL_INTERVAL);
    let short = Instant::now() + POLL_INTERVAL;
    assert_eq!(retry::probe_deadline(short), short);
    assert!(retry::pause(short).await.is_err());
}

#[test]
fn only_transport_unavailability_allows_a_startup_attempt() {
    for error in [
        ClientError::Deadline,
        ClientError::Protocol(ProtocolError::EndOfStream),
        ClientError::Protocol(ProtocolError::OutcomeUnknown),
        ClientError::Protocol(ProtocolError::Io(
            std::io::ErrorKind::ConnectionRefused.into(),
        )),
    ] {
        assert!(retry::transport_error(&error));
    }
    for error in [
        ClientError::IncompleteCatalogInventory,
        ClientError::Protocol(ProtocolError::ResponseMismatch),
        ClientError::Protocol(ProtocolError::ConnectionPoisoned),
    ] {
        assert!(!retry::transport_error(&error));
    }
}
