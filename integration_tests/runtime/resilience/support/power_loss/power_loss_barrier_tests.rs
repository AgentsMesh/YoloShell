use super::*;
use std::process::{Child, Command};
use tokio::sync::oneshot;

const TEST_PROCESS_EXECUTABLE: &str = "/bin/sleep";
const TEST_PROCESS_ARGUMENT: &str = "60";
const THAW_TIMEOUT: Duration = PROCESS_EXIT_TIMEOUT;
const INJECTED_FREEZE_FAILURE_DETAIL: &str = "injected failure after exact power-loss freeze";

struct BarrierTestProcess {
    identity: RuntimeProcessIdentity,
    child: Option<Child>,
}

impl BarrierTestProcess {
    fn spawn() -> Self {
        let child = Command::new(TEST_PROCESS_EXECUTABLE)
            .arg(TEST_PROCESS_ARGUMENT)
            .spawn()
            .unwrap();
        let identity = RuntimeProcessIdentity::capture(child.id()).unwrap();
        Self {
            identity,
            child: Some(child),
        }
    }

    fn settle(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        if let Some(mut child) = self.child.take() {
            child.wait().unwrap();
        }
    }
}

impl Drop for BarrierTestProcess {
    fn drop(&mut self) {
        self.settle();
    }
}

#[tokio::test]
async fn capture_to_signal_identity_loss_fails_closed() {
    let mut process = BarrierTestProcess::spawn();
    let identity = process.identity;
    let mut barrier = barrier_for(&[identity]);

    let result = barrier
        .freeze_after_capture_for_test(identity, || process.settle())
        .await;

    assert!(result.is_err());
    assert_eq!(identity.observe(), ProcessIdentityObservation::Gone);
}

#[tokio::test]
async fn multi_target_freeze_failure_drop_thaws_every_stopped_target() {
    let processes = [BarrierTestProcess::spawn(), BarrierTestProcess::spawn()];
    let identities = [processes[0].identity, processes[1].identity];
    assert!(
        freeze_then_fail(barrier_for(&identities), &identities)
            .await
            .is_err()
    );
    assert_all_thawed(&identities).await;
    drop(processes);
}

#[tokio::test]
async fn cancelling_multi_target_freeze_thaws_every_stopped_target() {
    let processes = [BarrierTestProcess::spawn(), BarrierTestProcess::spawn()];
    let identities = [processes[0].identity, processes[1].identity];
    let (frozen, frozen_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut barrier = barrier_for(&identities);
        for identity in identities {
            barrier.freeze_exact(identity).await.unwrap();
        }
        frozen.send(()).unwrap();
        std::future::pending::<()>().await;
        drop(barrier);
    });
    frozen_rx.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_all_thawed(&identities).await;
    drop(processes);
}

async fn assert_all_thawed(identities: &[RuntimeProcessIdentity]) {
    let deadline = tokio::time::Instant::now() + THAW_TIMEOUT;
    while identities
        .iter()
        .any(|identity| identity.is_stably_stopped().unwrap())
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }

    for identity in identities {
        assert_eq!(identity.observe(), ProcessIdentityObservation::Matching);
        assert!(!identity.is_stably_stopped().unwrap());
    }
}

async fn freeze_then_fail(
    mut barrier: PowerLossFreezeBarrier,
    identities: &[RuntimeProcessIdentity],
) -> Result<()> {
    for identity in identities {
        barrier.freeze_exact(*identity).await?;
        ensure!(identity.is_stably_stopped()?, TARGET_NOT_FROZEN_DETAIL);
    }
    anyhow::bail!(INJECTED_FREEZE_FAILURE_DETAIL)
}

fn barrier_for(identities: &[RuntimeProcessIdentity]) -> PowerLossFreezeBarrier {
    let terminal_id = Uuid::new_v4();
    PowerLossFreezeBarrier {
        workers: identities.to_vec(),
        guardians: Vec::new(),
        trees: Vec::new(),
        targets: identities
            .iter()
            .map(|identity| (*identity, PowerLossTargetOwner::Worker(terminal_id)))
            .collect(),
        frozen: HashSet::new(),
        thaw_on_drop: HashSet::new(),
        phase: PowerLossBarrierPhase::Freezing,
    }
}
