use super::*;

const TEST_SETTLEMENT_EPOCH_GRACE: Duration = Duration::from_secs(1);
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
const EXPECTED_FORCE_GRACE: Duration = Duration::from_millis(4_280);

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn force_grace_composes_inventory_empty_epoch_and_reap_budgets() {
    assert_eq!(
        process_discovery_force_grace(TEST_SETTLEMENT_EPOCH_GRACE, TEST_POLL_INTERVAL),
        EXPECTED_FORCE_GRACE
    );
}
