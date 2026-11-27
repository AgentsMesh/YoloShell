use super::*;
use crate::custody_guardian::settlement_containment::{
    NAMESPACE_LOSS_GRACEFUL_GRACE, NAMESPACE_LOSS_OBSERVATION_GRACE, NamespaceLossContainment,
};
use crate::process::ProcessIdentityObservation;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

#[test]
fn holder_arriving_between_quiescence_scan_and_finalize_is_tracked() {
    let _isolation = serialize_real_pty_containment_tests();
    let (mut fixture, _worker) = PrematureFixture::new(ProcessIdentityRelation::Gone);
    let slave = fixture.take_inherited_slave();
    let origin = tokio::time::Instant::now();
    let mut containment = NamespaceLossContainment::new(&fixture.runtime, origin);
    containment
        .reconcile_custody(&fixture.custody, origin)
        .unwrap();
    assert_eq!(fixture.terminate_child().signal(), Some(libc::SIGKILL));
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    containment
        .reconcile_custody(&fixture.custody, graceful)
        .unwrap();
    let holder = Rc::new(RefCell::new(None));
    let installed = Rc::clone(&holder);
    let _hook = crate::process::install_namespace_loss_pre_finalize_hook(move || {
        installed
            .borrow_mut()
            .replace(late_holder::LatePtyHolder::spawn(slave));
        Ok(())
    })
    .unwrap();
    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;

    assert!(
        super::reconcile_before(
            &fixture.custody,
            &mut containment,
            force,
            Instant::now() + fixture::TEST_TIMEOUT,
        )
        .is_none()
    );
    let identity = holder.borrow().as_ref().unwrap().identity();
    assert_eq!(identity.observe(), ProcessIdentityObservation::Matching);
    assert!(containment.tracks_holder(identity));
    let settled =
        super::wait_for_settlement(&fixture.custody, &fixture.runtime, &mut containment, force);
    assert_eq!(settled.worker, worker_identity(&fixture.runtime));
    assert_eq!(settled.child, child_identity(&fixture.runtime));
    holder.borrow_mut().as_mut().unwrap().wait_for_exit();
}
