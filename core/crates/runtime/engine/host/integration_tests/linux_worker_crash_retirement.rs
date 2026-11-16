use super::*;
use crate::pty_custody::{CustodyDescriptorSet, CustodyRuntimeIdentity};

#[path = "../test_support/integration/linux_worker_crash_retirement.rs"]
mod support;
use support::*;

#[path = "linux_worker_crash_retirement_adversarial.rs"]
mod adversarial;
#[path = "linux_worker_crash_retirement_pending.rs"]
mod pending;

#[tokio::test]
async fn linux_live_containment_remains_awaiting_settlement_without_signaling() {
    const FREEZE_CONTROL: &str = "cgroup.freeze";
    let (fixture, mut child) = Fixture::with_live_child();
    let record: process::linux_containment::LinuxContainmentRecord = fixture
        .host
        .namespace
        .read_json(&fixture.containment_path())
        .unwrap();
    let freeze_path = record
        .delegated_root
        .join(&record.leaf_name)
        .join(FREEZE_CONTROL);
    let before = std::fs::read(&freeze_path).unwrap();
    let fixture = fixture.reopen();
    let expected = CustodyRuntimeIdentity::from_record(&fixture.runtime);

    assert_eq!(
        fixture
            .host
            .secondary_custody
            .recovery_admission(&fixture.host, &expected),
        secondary_custody::SecondaryCustodyRecoveryAdmission::AwaitingSettlement
    );
    assert!(process_identity_is_verified_live(
        fixture.runtime.child_pid,
        fixture.runtime.child_start_time
    ));
    assert_eq!(std::fs::read(&freeze_path).unwrap(), before);

    child.stop();
    fixture.settle().await;
    assert_eq!(
        fixture
            .host
            .secondary_custody
            .recovery_admission(&fixture.host, &expected),
        secondary_custody::SecondaryCustodyRecoveryAdmission::Retired
    );
}

#[tokio::test]
async fn linux_crash_retirement_publishes_exact_receipt_and_retains_containment() {
    let fixture = Fixture::new();
    let containment_before = std::fs::read(fixture.containment_path()).unwrap();
    let armed_lease = fixture.armed.armed_lease().unwrap();

    fixture.settle().await;

    let retired = fixture.receipt();
    assert_eq!(retired.retired_lease(), Some(armed_lease));
    assert_eq!(
        armed_lease.descriptor_set,
        CustodyDescriptorSet::LinuxContainment
    );
    assert!(armed_lease.guardian.is_none());
    assert!(retired.retirement_evidence().unwrap().is_valid());
    assert_eq!(
        std::fs::read(fixture.containment_path()).unwrap(),
        containment_before
    );
    let evidence = fixture
        .host
        .secondary_custody
        .retirement_evidence(
            &fixture.host,
            &CustodyRuntimeIdentity::from_record(&fixture.runtime),
        )
        .unwrap();
    evidence.verify(&fixture.host).unwrap();
}

#[tokio::test]
async fn linux_crash_retirement_replays_after_host_restart_between_receipt_and_exit() {
    let fixture = Fixture::new();
    fixture.settle().await;
    let receipt_path = fixture.host.paths.pty_custody(fixture.runtime.terminal_id);
    let before: (crate::pty_custody::CustodyDurableRecord, _) = fixture
        .host
        .namespace
        .read_json_with_identity(&receipt_path)
        .unwrap();
    assert!(
        !fixture
            .host
            .paths
            .exit_record(fixture.runtime.terminal_id)
            .exists()
    );
    let fixture = fixture.reopen();

    fixture.publish_exit().await.unwrap();

    let after = fixture
        .host
        .namespace
        .read_json_with_identity(&receipt_path)
        .unwrap();
    assert_eq!(before, after);
    let exit = fixture.exit();
    assert_eq!(exit.reason, ExitReason::WorkerCrash);
    assert_eq!(exit.runtime_epoch, fixture.runtime.runtime_epoch);
    let fixture = fixture.reopen();
    fixture.publish_exit().await.unwrap();
    assert_eq!(fixture.exit(), exit);
}

#[tokio::test]
async fn linux_explicit_resume_settles_crash_before_watchdog_and_binding_selection() {
    let fixture = Fixture::new();
    let result = fixture
        .host
        .begin_resume_transaction(
            fixture.manifest.clone(),
            ResumeSelection::Preserve(ResumeIntent::Explicit),
            None,
        )
        .await;

    assert!(matches!(
        result,
        Err(SessionResumeFailure::TerminalRecreate(_))
    ));
    assert_eq!(fixture.exit().reason, ExitReason::WorkerCrash);
    assert_eq!(
        fixture.receipt().retired_lease(),
        fixture.armed.armed_lease()
    );
    let current = fixture
        .host
        .catalog_get(fixture.manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.recovery_state, RecoveryState::NeedsRecreate);
    assert!(
        fixture
            .host
            .runtime_is_settled(&fixture.runtime, Duration::ZERO)
            .await
    );
}

#[tokio::test]
async fn linux_explicit_resume_preserves_normal_exit_without_crash_receipt() {
    let fixture = Fixture::new();
    let natural = archive_exit_for(fixture.runtime.runtime_epoch);
    fixture
        .host
        .namespace
        .write_json(
            &fixture.host.paths.exit_record(fixture.runtime.terminal_id),
            &natural,
        )
        .unwrap();
    let _lifecycle = fixture
        .host
        .acquire_resume_lifecycle_lock(fixture.runtime.terminal_id, None)
        .await
        .unwrap();

    fixture
        .host
        .prepare_linux_crash_exit(&mut fixture.manifest.clone())
        .await
        .unwrap();

    assert_eq!(fixture.receipt(), fixture.armed);
    assert_eq!(fixture.exit(), natural);
}
