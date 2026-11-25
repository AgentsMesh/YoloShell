use super::*;
use crate::journal::Journal;
use crate::process::{PTY_SESSION_CAPABILITY_ENV, PtySessionProof};
use crate::worker::pty_fault_reaper::{FaultReapCompletion, FaultReapReceiver};
use crate::worker::pty_ownership::ReapEvidence;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::sync::Mutex;
use std::time::Instant;
use uuid::Uuid;

const BOOTSTRAP_FAILURE_JOURNAL: &str = "explicit-bootstrap-failure.journal";
const BOOTSTRAP_FAILURE_ROWS: u16 = 24;
const BOOTSTRAP_FAILURE_COLUMNS: u16 = 80;
const BOOTSTRAP_FAILURE_PIXEL_SIZE: u16 = 0;
const BOOTSTRAP_FAILURE_WAIT_TIMEOUT: std::time::Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;
const BOOTSTRAP_FAILURE_WAIT_INTERVAL: std::time::Duration =
    crate::worker::termination_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const BOOTSTRAP_FAILURE_EXIT_DETAIL: &str = "explicit bootstrap failure did not reap its root";

pub(super) enum FailureKind {
    Reported,
    CancelledBeforeRelease,
}

pub(super) struct ExplicitFailureFixture {
    _cleanup: ChildCleanup,
    pub(super) ownership: FaultReap,
    pub(super) root: ProcessIdentity,
    receiver: FaultReapReceiver,
    _directory: tempfile::TempDir,
}

impl ExplicitFailureFixture {
    pub(super) fn new(kind: FailureKind) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let pair = native_pty_system().openpty(pty_size()).unwrap();
        let capability = Uuid::new_v4().to_string();
        let mut proof =
            PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), capability.clone())
                .unwrap();
        let runtime_epoch = Uuid::new_v4();
        proof.bind_runtime(Uuid::new_v4(), runtime_epoch).unwrap();
        let transition = proof.take_bootstrap_exec_authority().unwrap();
        let mut command = CommandBuilder::new(EXCLUSIVE_SIGNAL_TEST_COMMAND);
        command.arg(EXCLUSIVE_SIGNAL_TEST_DURATION);
        command.env(PTY_SESSION_CAPABILITY_ENV, capability);
        let child = pair.slave.spawn_command(command).unwrap();
        let root = ProcessIdentity::capture(child.process_id().unwrap()).unwrap();
        let cleanup = ChildCleanup { root };
        drop(pair.slave);
        let release = proof.bind_master_alias_release(root).unwrap();
        let journal = Arc::new(Mutex::new(
            Journal::open(
                directory.path().join(BOOTSTRAP_FAILURE_JOURNAL),
                runtime_epoch,
            )
            .unwrap(),
        ));
        let mut output = ExclusivePtyOutput::capture(pair.master.as_ref(), journal).unwrap();
        output.bind_master_alias_authority(release).unwrap();
        let mut tree = CapturedProcessTree::capture_pty_for_test(root, proof).unwrap();
        let failure = match kind {
            FailureKind::Reported => transition.confirm_explicit_failure(),
            FailureKind::CancelledBeforeRelease => transition.cancel_before_release(),
        }
        .unwrap();
        tree.record_explicit_bootstrap_exec_failure(failure)
            .unwrap();
        let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
            .unwrap()
            .bind(child);
        let (completion, receiver) = FaultReapCompletion::owner_channel();
        let mut ownership =
            crate::worker::pty_fault_reaper::fault_reap_construction::passive_fault_reap_for_test();
        ownership.replace_completion_for_test(completion);
        drop(pair.master);
        output.mark_owner_master_released();
        let custody = ownership.custody_mut();
        custody.authority = Some(authority);
        custody.processes = Some(PtyProcessCustody::tracking(tree));
        custody.output_progress = FaultOutput::Exclusive(output);
        custody.access = PtyCustodyAccess::Control;
        Self {
            _cleanup: cleanup,
            ownership,
            root,
            receiver,
            _directory: directory,
        }
    }

    pub(super) fn assert_pending_before_exit(&mut self) {
        assert!(self.ownership.custody().root_exit_proof.is_none());
        assert!(
            !self
                .ownership
                .custody()
                .processes
                .as_ref()
                .unwrap()
                .requires_exact_root_force_fallback()
        );
        assert_eq!(
            bootstrap_failure::poll(&mut self.ownership).unwrap(),
            BootstrapFailurePoll::Pending
        );
        assert!(!self.ownership.completion().is_published());
    }

    pub(super) fn observe_root_exit(&mut self) {
        let deadline = Instant::now() + BOOTSTRAP_FAILURE_WAIT_TIMEOUT;
        while self.ownership.custody().root_exit_proof.is_none() {
            assert!(Instant::now() < deadline, "{BOOTSTRAP_FAILURE_EXIT_DETAIL}");
            live_authority::observe_root_exit(&mut self.ownership);
            std::thread::sleep(BOOTSTRAP_FAILURE_WAIT_INTERVAL);
        }
        assert!(
            self.ownership
                .custody()
                .authority
                .as_ref()
                .unwrap()
                .is_waited()
        );
    }

    pub(super) fn finish(&mut self, expected_signal: i32) {
        assert_eq!(
            bootstrap_failure::poll(&mut self.ownership).unwrap(),
            BootstrapFailurePoll::Retired
        );
        assert!(
            self.ownership
                .custody()
                .processes
                .as_ref()
                .unwrap()
                .session_is_retired()
        );
        assert!(
            self.ownership.custody().force.transaction_state() == ForceTransactionState::Unstarted
        );
        assert!(live_authority::finish_if_ready(&mut self.ownership));
        let outcome = self
            .receiver
            .receive_timeout(BOOTSTRAP_FAILURE_WAIT_TIMEOUT)
            .unwrap();
        assert_eq!(outcome.evidence, ReapEvidence::Waited);
        assert_eq!(outcome.status.unwrap().signal(), Some(expected_signal));
        assert!(outcome.fault.is_none());
        assert!(!self.root.matches());
    }
}

fn pty_size() -> PtySize {
    PtySize {
        rows: BOOTSTRAP_FAILURE_ROWS,
        cols: BOOTSTRAP_FAILURE_COLUMNS,
        pixel_width: BOOTSTRAP_FAILURE_PIXEL_SIZE,
        pixel_height: BOOTSTRAP_FAILURE_PIXEL_SIZE,
    }
}
