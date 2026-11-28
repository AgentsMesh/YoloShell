use yolo_hub_peer_diagnostics::{
    ExportDepthDenials, INBOUND_REJECTION_INTERVAL, InboundRejections, OutboundFailures,
    PEER_RETRY_INITIAL, PEER_RETRY_MAX, PeerAttempt, RetryDelay,
};
use std::time::Instant;
use yolo_hub_identity::HubId;

#[test]
fn outbound_failures_report_first_change_and_recovery_without_repeating() {
    let mut failures = OutboundFailures::default();
    assert_eq!(failures.fail(PeerAttempt::Dial), Some(1));
    assert_eq!(failures.fail(PeerAttempt::Dial), None);
    assert_eq!(failures.fail(PeerAttempt::Handshake), Some(3));
    assert_eq!(failures.fail(PeerAttempt::Handshake), None);
    assert_eq!(failures.succeed(), Some(4));
    assert_eq!(failures.succeed(), None);
}

#[test]
fn outbound_failures_report_periodically_during_a_long_outage() {
    let mut failures = OutboundFailures::default();
    let reports: Vec<_> = (1..=FAILURE_REPORTS)
        .filter_map(|_| failures.fail(PeerAttempt::Admission))
        .collect();
    assert_eq!(reports, vec![1, 30]);
    assert_eq!(failures.succeed(), Some(30));
}

const FAILURE_REPORTS: u64 = 30;

#[test]
fn peer_retry_delay_backs_off_without_a_missed_tick_burst() {
    let mut retry = RetryDelay::default();
    assert_eq!(retry.delay(), PEER_RETRY_INITIAL);
    retry.failed();
    assert_eq!(retry.delay(), PEER_RETRY_INITIAL * 2);
    for _ in 0..8 {
        retry.failed();
    }
    assert_eq!(retry.delay(), PEER_RETRY_MAX);
    retry.reset();
    assert_eq!(retry.delay(), PEER_RETRY_INITIAL);
}

#[test]
fn inbound_rejections_are_window_bounded() {
    let mut rejections = InboundRejections::default();
    let now = Instant::now();
    assert_eq!(rejections.reject(now), Some(0));
    assert_eq!(rejections.reject(now), None);
    assert_eq!(rejections.reject(now), None);
    assert_eq!(
        rejections.reject(now + INBOUND_REJECTION_INTERVAL),
        Some(2)
    );
    assert_eq!(rejections.reject(now + INBOUND_REJECTION_INTERVAL), None);
    assert_eq!(
        rejections.reject(now + INBOUND_REJECTION_INTERVAL * 2),
        Some(1)
    );
}

#[test]
fn depth_denials_report_once_per_peer_origin_and_epoch() {
    let denials = ExportDepthDenials::default();
    let (peer, origin) = (HubId::new(), HubId::new());
    assert!(denials.denied(peer, origin, 7));
    assert!(!denials.denied(peer, origin, 7));
    assert!(denials.denied(peer, origin, 8));
    assert!(denials.denied(peer, HubId::new(), 7));
    assert!(denials.denied(HubId::new(), origin, 7));
}

#[test]
fn a_successful_export_rearms_the_denial_report() {
    let denials = ExportDepthDenials::default();
    let (peer, origin) = (HubId::new(), HubId::new());
    assert!(denials.denied(peer, origin, 7));
    denials.exported(peer, origin, 7);
    assert!(denials.denied(peer, origin, 7));
}
