use super::*;
use crate::custody_guardian::settlement_containment::NamespaceLossContainment;

#[test]
fn observing_known_targets_reuses_the_initial_holder_discovery() {
    let _isolation = serialize_real_pty_containment_tests();
    let (fixture, _worker) = Fixture::new(ProcessIdentityRelation::Gone);
    let origin = tokio::time::Instant::now();
    let mut containment = NamespaceLossContainment::new(&fixture.runtime, origin);
    let deadline = Instant::now() + TEST_TIMEOUT;
    assert!(
        super::reconcile_before(&fixture.custody, &mut containment, origin, deadline).is_none()
    );
    let initial_scans = containment.holder_discovery_scans_for_test();

    assert!(
        containment
            .reconcile_custody(&fixture.custody, origin)
            .unwrap()
            .is_none()
    );
    assert!(
        containment
            .reconcile_custody(&fixture.custody, origin)
            .unwrap()
            .is_none()
    );
    assert_eq!(containment.holder_discovery_scans_for_test(), initial_scans);
}
