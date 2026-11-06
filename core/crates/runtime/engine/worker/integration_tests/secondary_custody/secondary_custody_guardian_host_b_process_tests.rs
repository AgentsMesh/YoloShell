use super::spawn;
use super::support::{DetachedGuardian, Fixture, HostSuccessor, RuntimeFence};
use crate::pty_custody::{CustodyDurableSnapshot, CustodyDurableState};
use std::time::Duration;

const ACTIVATION_FENCE_OBSERVATION: Duration = Duration::from_millis(150);

#[tokio::test]
async fn same_size_resize_publication_preserves_live_guardian_host_replacement() {
    let mut fixture = Fixture::new();
    let preparation_a = spawn(&fixture).await;
    let guardian = DetachedGuardian::capture(preparation_a);
    let prepared_a = fixture.current().0;
    fixture.arm(preparation_a, &fixture.request, &fixture.offer);
    let runtime_before_resize = fixture.current_runtime_snapshot();

    crate::worker::publish_runtime_record(
        &fixture.namespace,
        &fixture.paths,
        &runtime_before_resize.0,
    )
    .unwrap();
    assert_eq!(fixture.current_runtime_snapshot(), runtime_before_resize);
    fixture.terminate_host();
    let restored_a = fixture.wait_for_record(&prepared_a);

    let host_b = HostSuccessor::capture();
    let (request_b, offer_b) = host_b.exchange(&fixture);
    let preparation_b = crate::custody_guardian::prepare_existing(
        &fixture.paths,
        preparation_a.binding(),
        &request_b,
        &offer_b,
        fixture.runtime_identity,
        &CustodyDurableSnapshot::new(restored_a.0, restored_a.1),
    )
    .await
    .unwrap();
    let armed_b = fixture.arm(preparation_b, &request_b, &offer_b);
    let promoted_b = fixture.publish_worker_runtime_promotion(&armed_b);

    assert_eq!(
        promoted_b.controller_generation,
        request_b.controller_generation
    );
    fixture.assert_worker_and_child_current();
    guardian.assert_current();
}

#[tokio::test]
async fn live_guardian_restores_prepared_a_and_reuses_it_for_host_b() {
    let mut fixture = Fixture::new();
    let runtime_before = fixture.current_runtime_snapshot();
    let preparation_a = spawn(&fixture).await;
    let guardian = DetachedGuardian::capture(preparation_a);
    let (prepared_a, _) =
        fixture.arm_a_then_host_exit(preparation_a, RuntimeFence::Retained, || {});

    let restored_a = fixture.wait_for_record(&prepared_a);
    assert_ne!(restored_a.1, preparation_a.custody_file_identity());
    assert_eq!(fixture.current_runtime_snapshot(), runtime_before);
    fixture.assert_worker_and_child_current();
    guardian.assert_current();

    let host_b = HostSuccessor::capture();
    assert_ne!(host_b.holder(), preparation_a.binding().host_holder());
    let (request_b, offer_b) = host_b.exchange(&fixture);
    let predecessor_b = CustodyDurableSnapshot::new(restored_a.0, restored_a.1);
    let preparation_b = crate::custody_guardian::prepare_existing(
        &fixture.paths,
        preparation_a.binding(),
        &request_b,
        &offer_b,
        fixture.runtime_identity,
        &predecessor_b,
    )
    .await
    .unwrap();
    assert_eq!(
        preparation_b.binding().process(),
        preparation_a.binding().process()
    );
    assert_eq!(
        preparation_b.binding().guardian_id(),
        preparation_a.binding().guardian_id()
    );

    let (prepared_b, prepared_b_identity) = fixture.current();
    assert_eq!(prepared_b_identity, preparation_b.custody_file_identity());
    let CustodyDurableState::PreparedGuardian {
        predecessor,
        candidate,
    } = &prepared_b.state
    else {
        panic!("host B did not install a single PreparedGuardian");
    };
    assert_eq!(predecessor.as_ref(), &fixture.predecessor.record().state);
    assert!(!matches!(
        predecessor.as_ref(),
        CustodyDurableState::PreparedGuardian { .. }
    ));
    assert!(preparation_b.binding().matches_lease_identity(candidate));

    let armed_b = fixture.arm(preparation_b, &request_b, &offer_b);
    let awaiting_activation = fixture.current_runtime_snapshot();
    assert_eq!(awaiting_activation.0, fixture.runtime);
    assert_eq!(awaiting_activation.1, fixture.runtime_identity);
    std::thread::sleep(ACTIVATION_FENCE_OBSERVATION);
    assert_eq!(fixture.current().0, armed_b);
    assert_eq!(fixture.current_runtime_snapshot(), awaiting_activation);

    let promoted_b = fixture.publish_worker_runtime_promotion(&armed_b);
    assert_eq!(
        promoted_b.controller_generation,
        request_b.controller_generation
    );
    let promoted_snapshot = fixture.current_runtime_snapshot();
    assert_eq!(promoted_snapshot.0, promoted_b);
    assert_ne!(promoted_snapshot.1, awaiting_activation.1);
    assert_eq!(fixture.current().0, armed_b);
    fixture.assert_worker_and_child_current();
    guardian.assert_current();
}

#[tokio::test]
async fn dead_guardian_is_replaced_for_host_b_without_killing_worker_or_pty() {
    let mut fixture = Fixture::new();
    let preparation_a = spawn(&fixture).await;
    let mut guardian_a = DetachedGuardian::capture(preparation_a);
    let (_, armed_a) = fixture.arm_a_then_host_exit(preparation_a, RuntimeFence::Retained, || {
        guardian_a.terminate()
    });
    let armed_a_snapshot = fixture.current();
    assert_eq!(armed_a_snapshot.0, armed_a);

    let host_b = HostSuccessor::capture();
    assert_ne!(host_b.holder(), preparation_a.binding().host_holder());
    let (request_b, offer_b) = host_b.exchange(&fixture);
    let preparation_b = crate::custody_guardian::spawn_successor(
        &fixture.executable(),
        &fixture.paths,
        fixture.runtime.terminal_id,
        &request_b,
        &offer_b,
        fixture.runtime_identity,
        CustodyDurableSnapshot::new(armed_a_snapshot.0, armed_a_snapshot.1),
        &fixture.descriptors(),
    )
    .await
    .unwrap();
    let guardian_b = DetachedGuardian::capture(preparation_b);
    assert_ne!(
        preparation_b.binding().process(),
        preparation_a.binding().process()
    );

    let (prepared_b, prepared_b_identity) = fixture.current();
    assert_eq!(prepared_b_identity, preparation_b.custody_file_identity());
    let CustodyDurableState::PreparedGuardian { predecessor, .. } = &prepared_b.state else {
        panic!("replacement guardian did not install PreparedGuardian");
    };
    assert_eq!(predecessor.as_ref(), &armed_a.state);
    assert!(!matches!(
        predecessor.as_ref(),
        CustodyDurableState::PreparedGuardian { .. }
    ));

    let armed_b = fixture.arm(preparation_b, &request_b, &offer_b);
    let promoted_b = fixture.publish_worker_runtime_promotion(&armed_b);
    assert_eq!(
        promoted_b.controller_generation,
        request_b.controller_generation
    );
    assert_eq!(fixture.current().0, armed_b);
    fixture.assert_worker_and_child_current();
    guardian_b.assert_current();
}
