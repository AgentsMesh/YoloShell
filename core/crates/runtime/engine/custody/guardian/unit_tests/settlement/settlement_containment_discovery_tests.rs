use super::*;

#[test]
fn finalized_discovery_uses_a_capped_backoff_and_resets_after_a_hit() {
    let origin = Instant::now();
    let mut discovery = LateHolderDiscovery::default();
    discovery.arm(origin);
    assert!(!discovery.should_scan(origin));

    let first = origin + INITIAL_DISCOVERY_BACKOFF;
    assert!(discovery.should_scan(first));
    discovery.record_success(first, false);
    assert!(!discovery.should_scan(first));

    let second = first + INITIAL_DISCOVERY_BACKOFF;
    assert!(discovery.should_scan(second));
    discovery.record_success(second, true);
    assert!(!discovery.should_scan(second));
    assert!(discovery.should_scan(second + INITIAL_DISCOVERY_BACKOFF));

    let mut now = second;
    while discovery.delay < MAXIMUM_DISCOVERY_BACKOFF {
        now += discovery.delay;
        discovery.record_success(now, false);
    }
    assert_eq!(discovery.delay, MAXIMUM_DISCOVERY_BACKOFF);
}

#[test]
fn failed_scan_remains_due_for_the_next_poll() {
    let origin = Instant::now();
    let mut discovery = LateHolderDiscovery::default();
    discovery.arm(origin);
    let due = origin + INITIAL_DISCOVERY_BACKOFF;

    assert!(discovery.should_scan(due));
    assert!(discovery.should_scan(due));
    discovery.record_success(due, false);
    assert!(!discovery.should_scan(due));
}
