use super::spawn;
use super::support::{DetachedGuardian, Fixture, RuntimeFence};
use std::time::Duration;

const FAIL_CLOSED_OBSERVATION: Duration = Duration::from_millis(150);

#[tokio::test]
async fn same_value_new_inode_runtime_fences_armed_to_prepared_restoration() {
    let mut fixture = Fixture::new();
    let preparation = spawn(&fixture).await;
    let guardian = DetachedGuardian::capture(preparation);
    let (_, armed) =
        fixture.arm_a_then_host_exit(preparation, RuntimeFence::SameValueNewIdentity, || {});
    let armed_snapshot = fixture.current();
    assert_eq!(armed_snapshot.0, armed);

    std::thread::sleep(FAIL_CLOSED_OBSERVATION);
    assert_eq!(fixture.current(), armed_snapshot);
    let current_runtime = fixture.current_runtime_snapshot();
    assert_eq!(current_runtime.0, fixture.runtime);
    assert_ne!(current_runtime.1, fixture.runtime_identity);
    fixture.assert_worker_and_child_current();
    guardian.assert_current();
}

#[tokio::test]
async fn promoted_runtime_wins_and_never_rolls_armed_back_to_prepared() {
    let mut fixture = Fixture::new();
    let preparation = spawn(&fixture).await;
    let guardian = DetachedGuardian::capture(preparation);
    let armed = fixture.arm(preparation, &fixture.request, &fixture.offer);
    let armed_snapshot = fixture.current();
    assert_eq!(armed_snapshot.0, armed);
    let promoted = crate::custody_promotion::publish_exact(
        fixture.namespace.lease(),
        &fixture.paths,
        &armed_snapshot.0,
        armed_snapshot.1,
    )
    .unwrap();
    assert_eq!(
        promoted.controller_generation,
        fixture.request.controller_generation
    );
    let promoted_snapshot = fixture.current_runtime_snapshot();
    assert_eq!(promoted_snapshot.0, promoted);
    assert_ne!(promoted_snapshot.1, fixture.runtime_identity);

    fixture.terminate_host();
    std::thread::sleep(FAIL_CLOSED_OBSERVATION);
    assert_eq!(fixture.current(), armed_snapshot);
    assert_eq!(fixture.current_runtime_snapshot(), promoted_snapshot);
    fixture.assert_worker_and_child_current();
    guardian.assert_current();
}
