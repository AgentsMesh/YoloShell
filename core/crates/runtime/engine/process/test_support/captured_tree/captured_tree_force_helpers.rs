use super::{
    FreezePoll, FreezingProcessTree, FrozenKillBasis, FrozenProcessTree, KernelEofKillReady,
    KillCommitPoll, KillPendingProcessTree,
};
#[cfg(target_os = "macos")]
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::time::{Duration, Instant};

const KILL_RETRY_TIMEOUT: Duration = Duration::from_secs(5);
const KILL_RETRY_INTERVAL: Duration = Duration::from_millis(10);
const KILL_RETRY_TIMEOUT_DETAIL: &str = "frozen process tree did not become killable";
const EXPECTED_KILL_FAILURE_DETAIL: &str =
    "frozen kill completed without reaching the injected failure";
const UNEXPECTED_KILL_ERROR_DETAIL: &str = "frozen kill returned an unexpected error";
const KERNEL_EOF_AUTHORITY_REQUIRED_DETAIL: &str = "kernel EOF test authority is unavailable";
#[cfg(target_os = "macos")]
const LIVE_TEST_PROCESS_REQUIRED_DETAIL: &str = "test process identity is not live";

pub(super) fn kill_with_retry(mut frozen: FrozenProcessTree) -> KillPendingProcessTree {
    let deadline = Instant::now() + KILL_RETRY_TIMEOUT;
    loop {
        let version = frozen.closure_version();
        match frozen.kill_after_output_fence(version) {
            Ok(KillCommitPoll::Pending(pending)) => return pending,
            Ok(KillCommitPoll::Refreeze(freezing)) => {
                frozen = freeze_until(freezing, deadline);
            }
            Err(error) => {
                assert!(
                    Instant::now() < deadline,
                    "{UNEXPECTED_KILL_ERROR_DETAIL}: {error:?}"
                );
                frozen = error.into_state();
            }
        }
        assert!(
            Instant::now() < deadline,
            "{KILL_RETRY_TIMEOUT_DETAIL}: {:?}",
            frozen.state_snapshot_for_test()
        );
        std::thread::sleep(KILL_RETRY_INTERVAL);
    }
}

pub(super) fn freeze_with_retry(freezing: FreezingProcessTree) -> FrozenProcessTree {
    freeze_until(freezing, Instant::now() + KILL_RETRY_TIMEOUT)
}

pub(super) fn kill_error_with_retry(
    mut frozen: FrozenProcessTree,
) -> crate::process::ProcessTransitionError<FrozenProcessTree> {
    let deadline = Instant::now() + KILL_RETRY_TIMEOUT;
    loop {
        let version = frozen.closure_version();
        match frozen.kill_after_output_fence(version) {
            Err(error) => return error,
            Ok(KillCommitPoll::Refreeze(freezing)) => frozen = freeze_until(freezing, deadline),
            Ok(KillCommitPoll::Pending(_)) => panic!("{EXPECTED_KILL_FAILURE_DETAIL}"),
        }
        assert!(Instant::now() < deadline, "{KILL_RETRY_TIMEOUT_DETAIL}");
        std::thread::sleep(KILL_RETRY_INTERVAL);
    }
}

fn freeze_until(mut freezing: FreezingProcessTree, deadline: Instant) -> FrozenProcessTree {
    loop {
        match freezing.poll() {
            Ok(FreezePoll::Pending(next)) => freezing = next,
            Ok(FreezePoll::Frozen(frozen)) => return frozen,
            Err(error) => freezing = error.into_state(),
        }
        assert!(
            Instant::now() < deadline,
            "{KILL_RETRY_TIMEOUT_DETAIL}: {:?}",
            freezing.state_snapshot_for_test()
        );
        std::thread::sleep(KILL_RETRY_INTERVAL);
    }
}

impl FrozenProcessTree {
    pub fn install_kernel_eof_authority_for_test(
        &mut self,
        permit: crate::process::kernel_eof_authority::KernelEofPermit,
    ) {
        self.kill_basis = FrozenKillBasis::KernelEof;
        self.kernel_eof = Some(
            permit
                .authorize(&self.version)
                .expect(KERNEL_EOF_AUTHORITY_REQUIRED_DETAIL),
        );
    }
}

#[cfg(target_os = "macos")]
impl FrozenProcessTree {
    pub fn downgrade_kill_basis_to_ordinary_for_test(&mut self) {
        self.kill_basis = FrozenKillBasis::Ordinary;
    }

    pub fn force_kernel_eof_closure_stable_for_test(&mut self) {
        self.tree.test_state.force_kernel_eof_closure_stable();
    }

    pub fn inject_unresolved_fork_for_test(&mut self, source: ProcessIdentity) {
        self.tree
            .events
            .inject_unresolved_fork(live_for_test(source));
    }

    pub fn fail_next_event_read_for_test(&mut self) {
        self.tree.events.fail_next_read();
    }

    pub fn validated_kernel_eof_for_test(
        &self,
    ) -> crate::process::kernel_eof_authority::ValidatedKernelEof {
        self.kernel_eof
            .as_ref()
            .expect(KERNEL_EOF_AUTHORITY_REQUIRED_DETAIL)
            .validated_for_test()
    }

    pub fn seal_kernel_eof_for_test(
        &self,
        ready: &KernelEofKillReady,
        validated: crate::process::kernel_eof_authority::ValidatedKernelEof,
    ) -> crate::process::kernel_eof_authority::ValidatedSealedKernelEof {
        validated.bind_reopen_seal_for_test(&ready.seal)
    }

    pub fn kernel_eof_permit_for_test(
        &self,
    ) -> crate::process::kernel_eof_authority::KernelEofPermit {
        let output = crate::process::kernel_eof_authority::KernelPtyEofIdentity::from_kernel_read();
        crate::process::kernel_eof_authority::bind_scripted_kernel_eof(&self.version, &output).0
    }

    pub fn inject_captured_producer_for_test(&mut self, durable: ProcessIdentity) {
        self.tree.captured.insert(live_for_test(durable), durable);
    }

    pub fn reopen_seal_is_quarantined_for_test(&self) -> bool {
        self.tree
            .pty_proof
            .as_ref()
            .is_some_and(crate::process::PtySessionProof::reopen_seal_is_quarantined_for_test)
    }
}

#[cfg(target_os = "macos")]
fn live_for_test(identity: ProcessIdentity) -> LiveProcessIdentity {
    LiveProcessIdentity::capture(identity)
        .unwrap()
        .expect(LIVE_TEST_PROCESS_REQUIRED_DETAIL)
}
