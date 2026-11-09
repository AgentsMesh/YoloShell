use super::*;
use crate::custody_guardian::settlement_containment::{
    NAMESPACE_LOSS_GRACEFUL_GRACE, NAMESPACE_LOSS_OBSERVATION_GRACE, NamespaceLossContainment,
    NamespaceLossExactSettlement,
};

const UNEXPECTED_REFINALIZATION_DETAIL: &str =
    "finalized namespace-loss seal was recomputed during drain retry";

#[test]
fn finalized_seal_is_retained_when_the_first_kernel_eof_drain_is_pending() {
    let _isolation = serialize_real_pty_containment_tests();
    let (mut fixture, _worker) = PrematureFixture::new(ProcessIdentityRelation::Gone);
    drop(fixture.take_inherited_slave());
    let origin = tokio::time::Instant::now();
    let mut containment = NamespaceLossContainment::new(&fixture.runtime, origin);
    containment
        .reconcile_custody(&fixture.custody, origin)
        .unwrap();
    assert_eq!(fixture.terminate_child().signal(), Some(libc::SIGKILL));
    assert!(
        namespace_processes_settled(&fixture.custody, &fixture.runtime, None)
            .unwrap()
            .is_none()
    );
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    containment
        .reconcile_custody(&fixture.custody, graceful)
        .unwrap();
    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;
    let exact = wait_for_exact_containment(&fixture.custody, &mut containment, force);
    assert!(matches!(
        capability::exact_settlement(&fixture.custody).unwrap(),
        capability::ExactSettlement::PrematurelyReleased
    ));
    let (foreign, _foreign_worker) = PrematureFixture::new(ProcessIdentityRelation::Gone);
    assert!(
        foreign
            .custody
            .drain_namespace_loss_output_to_kernel_eof(exact.finalized())
            .is_err()
    );
    let _pending = crate::process::install_namespace_loss_drain_pending_once();
    assert!(
        namespace_processes_settled(&fixture.custody, &fixture.runtime, Some(exact))
            .unwrap()
            .is_none()
    );
    let _refinalization = crate::process::install_namespace_loss_pre_finalize_hook(|| {
        anyhow::bail!(UNEXPECTED_REFINALIZATION_DETAIL)
    })
    .unwrap();
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let exact = containment
            .reconcile_custody(&fixture.custody, force)
            .unwrap()
            .unwrap();
        if namespace_processes_settled(&fixture.custody, &fixture.runtime, Some(exact))
            .unwrap()
            .is_some()
        {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_for_exact_containment(
    custody: &MacosSecondaryCustody,
    containment: &mut NamespaceLossContainment,
    now: tokio::time::Instant,
) -> NamespaceLossExactSettlement {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if let Some(exact) = containment.reconcile_custody(custody, now).unwrap() {
            return exact;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}
