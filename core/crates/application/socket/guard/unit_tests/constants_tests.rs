use super::*;

#[test]
fn completion_deadline_exceeds_the_server_settlement_budget() {
    assert!(WATCHDOG_COMPLETION_MARGIN > Duration::ZERO);
    assert!(WATCHDOG_COMPLETION_TIMEOUT > WATCHDOG_SERVER_SETTLEMENT_BUDGET);
    assert_eq!(
        WATCHDOG_SERVER_SETTLEMENT_BUDGET,
        WATCHDOG_DISCOVERY_PREPARATION_TIMEOUT
            .saturating_add(GRACEFUL_CLEANUP_WINDOW)
            .saturating_add(WATCHDOG_FORCE_SETTLEMENT_TIMEOUT)
    );
}
