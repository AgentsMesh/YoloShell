use super::*;
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
use yolo_resilience_support::secondary_custody_adversarial_observation::ActiveCustodyObservation;

#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
pub(super) const GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT: std::time::Duration =
    yolo_resilience_support::secondary_custody_adversarial_observation::GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT;
#[cfg(not(all(target_os = "macos", feature = "adversarial-test-hooks")))]
pub(super) const GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT: std::time::Duration =
    PROCESS_EXIT_TIMEOUT;

#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
const GUARDIAN_OBSERVATION_TIMEOUT_DETAIL: &str =
    "Host did not publish exact Guardian custody evidence";

pub(super) struct OrphanCleanup {
    identities: Vec<ProcessIdentity>,
}

impl OrphanCleanup {
    pub(super) fn processes(identities: impl IntoIterator<Item = ProcessIdentity>) -> Self {
        Self {
            identities: identities.into_iter().collect(),
        }
    }

    pub(super) fn track(&mut self, identity: ProcessIdentity) {
        self.identities.push(identity);
    }
}

impl Drop for OrphanCleanup {
    fn drop(&mut self) {
        let mut killed = Vec::new();
        for identity in &self.identities {
            if identity.observe() != ProcessIdentityObservation::Gone
                && matches!(identity.signal(libc::SIGKILL), Ok(true))
            {
                killed.push(*identity);
            }
        }
        wait_for_forced_cleanup(&self.identities);
        for identity in killed {
            if identity.observe() == ProcessIdentityObservation::Gone {
                discard_incomplete_coverage_profile(identity.pid);
            }
        }
    }
}

fn wait_for_forced_cleanup(identities: &[ProcessIdentity]) {
    let deadline = std::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while identities
        .iter()
        .any(|identity| identity.observe() != ProcessIdentityObservation::Gone)
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

pub(super) async fn wait_for_orphan_settlement(
    worker: ProcessIdentity,
    child: ProcessIdentity,
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))] guardian: ProcessIdentity,
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    guardian_custody: yolo_process_test_support::PtyMasterFdCustodyProbe,
) {
    loop {
        let worker_state = worker.observe();
        let child_state = child.observe();
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        assert_guardian_retained(worker, child, guardian, &guardian_custody);
        let runtime_gone = worker_state == ProcessIdentityObservation::Gone
            && child_state == ProcessIdentityObservation::Gone;
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        let all_gone = runtime_gone && guardian.observe() == ProcessIdentityObservation::Gone;
        #[cfg(not(all(target_os = "macos", feature = "adversarial-test-hooks")))]
        let all_gone = runtime_gone;
        if all_gone {
            return;
        }
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
fn assert_guardian_retained(
    worker: ProcessIdentity,
    child: ProcessIdentity,
    guardian: ProcessIdentity,
    custody: &yolo_process_test_support::PtyMasterFdCustodyProbe,
) {
    let guardian_state = guardian.observe();
    let guardian_custody = custody.process_holds_pty_object(guardian.pid);
    let runtime_still_alive = worker.observe() != ProcessIdentityObservation::Gone
        || child.observe() != ProcessIdentityObservation::Gone;
    if runtime_still_alive {
        assert_eq!(guardian_state, ProcessIdentityObservation::Matching);
        assert!(
            guardian_custody.as_ref().is_ok_and(|held| *held),
            "exact Guardian lost PTY custody: {guardian_custody:?}"
        );
    }
}

#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
pub(super) async fn active_guardian(
    harness: &Harness,
    terminal_id: Uuid,
    generation: u64,
) -> (
    ProcessIdentity,
    yolo_process_test_support::PtyMasterFdCustodyProbe,
) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    loop {
        if let Ok(active) = ActiveCustodyObservation::read(&harness.paths(), terminal_id)
            && active.controller_generation() == generation
        {
            let guardian = active.guardian();
            return (
                ProcessIdentity {
                    pid: guardian.pid,
                    start_time: guardian.start_time,
                },
                active.guardian_custody(),
            );
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{GUARDIAN_OBSERVATION_TIMEOUT_DETAIL}: {}",
            harness.host_stderr_tail()
        );
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

pub(super) fn guardian_diagnostic(
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    observation: ProcessIdentityObservation,
) -> Option<ProcessIdentityObservation> {
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    return Some(observation);
    #[cfg(not(all(target_os = "macos", feature = "adversarial-test-hooks")))]
    None
}
