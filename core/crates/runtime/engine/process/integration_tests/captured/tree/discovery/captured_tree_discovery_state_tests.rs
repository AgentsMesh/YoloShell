use super::*;

#[test]
fn inactive_discovery_ignores_unrequested_inventory_coverage() {
    let mut epoch = DiscoveryScanEpoch::Inactive;

    epoch.observe(DiscoveryEpochProgress::covered(false, true));

    assert_eq!(epoch, DiscoveryScanEpoch::Inactive);
    assert!(!epoch.coverage_is_complete());
}

#[test]
fn complete_coverage_does_not_claim_open_writer_settlement() {
    let mut epoch = DiscoveryScanEpoch::Pending;

    epoch.observe(DiscoveryEpochProgress::coverage_only(true));

    assert_eq!(epoch, DiscoveryScanEpoch::Covered);
    assert!(epoch.coverage_is_complete());
}

#[test]
fn incomplete_current_scan_revokes_prior_coverage() {
    let mut epoch = DiscoveryScanEpoch::Covered;

    epoch.observe(DiscoveryEpochProgress::default());

    assert_eq!(epoch, DiscoveryScanEpoch::Pending);
    assert!(!epoch.coverage_is_complete());
}

#[test]
fn released_writer_can_settle_a_covered_epoch() {
    let mut epoch = DiscoveryScanEpoch::Covered;

    epoch.observe(DiscoveryEpochProgress::covered(false, true));

    assert_eq!(epoch, DiscoveryScanEpoch::Complete);
    assert!(epoch.coverage_is_complete());
}

#[test]
fn completed_epoch_rejects_later_inventory_observations() {
    let mut epoch = DiscoveryScanEpoch::Complete;

    assert!(!epoch.requested());
    assert!(!epoch.observe(DiscoveryEpochProgress::default()));

    assert_eq!(epoch, DiscoveryScanEpoch::Complete);
}
