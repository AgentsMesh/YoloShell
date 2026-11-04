use super::fence_test_support::Fixture;
use super::types::CustodyPublication;
use super::*;

const DIFFERENT_TARGET_PID_OFFSET: u32 = 1;
const FIRST_RECORD: usize = 0;
const NOT_ARCHIVED: bool = false;
const TEST_HOME_LABEL: &str = "Claude custody startup receipt test home";

#[test]
fn startup_receipt_requires_active_exact_process_authority() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    let namespace =
        crate::namespace_lease::NamespaceLease::capture(&[(TEST_HOME_LABEL, fixture.paths.home())])
            .unwrap();
    let target = fixture.spec.target.process();
    let worker = fixture.spec.worker.process();
    let boot_id = fixture.spec.target.boot_id.clone();

    assert!(!receipt(&namespace, &fixture, &boot_id, target, worker));
    arm(&fixture.ledger, &fixture.spec).unwrap();
    assert!(!receipt(&namespace, &fixture, &boot_id, target, worker));
    persistence::update(&fixture.ledger, |ledger| {
        ledger.records[FIRST_RECORD].publication = CustodyPublication::Active;
        Ok(())
    })
    .unwrap();

    assert!(receipt(&namespace, &fixture, &boot_id, target, worker));
    let different_target = crate::process::ProcessIdentity {
        pid: target.pid.saturating_add(DIFFERENT_TARGET_PID_OFFSET),
        start_time: target.start_time,
    };
    assert!(!receipt(
        &namespace,
        &fixture,
        &boot_id,
        different_target,
        worker,
    ));
    mark_containing(&fixture.ledger, &fixture.spec).unwrap();
    assert!(!receipt(&namespace, &fixture, &boot_id, target, worker));
}

fn receipt(
    namespace: &crate::namespace_lease::NamespaceLease,
    fixture: &Fixture,
    boot_id: &str,
    target: crate::process::ProcessIdentity,
    worker: crate::process::ProcessIdentity,
) -> bool {
    has_active_startup_receipt(namespace, &fixture.ledger, boot_id, target, worker).unwrap()
}
