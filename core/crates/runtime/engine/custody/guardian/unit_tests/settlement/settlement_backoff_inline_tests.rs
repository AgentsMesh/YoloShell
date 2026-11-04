use super::*;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn persistent_failures_use_capped_exponential_backoff() {
    let origin = Instant::now();
    let mut backoff = FailureLogBackoff::default();
    let mut now = origin;

    while backoff.delay() < MAXIMUM_FAILURE_LOG_BACKOFF {
        let delay = backoff.delay();
        assert!(backoff.should_emit(now));
        assert!(!backoff.should_emit(now));
        now += delay;
    }
    assert_eq!(backoff.delay(), MAXIMUM_FAILURE_LOG_BACKOFF);
    assert!(backoff.should_emit(now));
    now += MAXIMUM_FAILURE_LOG_BACKOFF;
    assert!(backoff.should_emit(now));
    assert_eq!(backoff.delay(), MAXIMUM_FAILURE_LOG_BACKOFF);
}
