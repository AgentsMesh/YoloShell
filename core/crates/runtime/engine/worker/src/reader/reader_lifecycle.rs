use super::pty_output_progress::OutputProgressState;
use super::reader_restart::{ReaderRestartKind, ReaderRestartPhase};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[cfg(test)]
const RESTART_WAIT_PROBE_INSTALLED_DETAIL: &str = "reader restart wait probe is already installed";

#[derive(Debug, Default)]
pub(crate) struct ReaderProgress {
    pub(crate) active: bool,
    pub(crate) eof: eof_state::ReaderEofState,
    pub(crate) kernel_eof_probe_identity:
        Option<Arc<super::pty_output_progress::KernelPtyEofIdentity>>,
    pub(crate) kernel_eof_seal_epoch:
        Option<super::pty_output_progress::CommittedKernelEofSealEpoch>,
    pub(crate) read_stopped: bool,
    pub(crate) producers_gone: bool,
    pub(crate) output_read_in_flight: bool,
    pub(crate) output_abandoned: bool,
    pub(crate) namespace_loss_drain: namespace_loss::NamespaceLossDrainState,
    pub(crate) pending: Vec<PendingOutput>,
    pub(crate) pending_commit_retries: super::PendingCommitRetryBudget,
    reader_supervisor_completion: Option<supervisor_completion::ReaderSupervisorCompletion>,
    pub(crate) requested_probe_epoch: quiescence::OutputProbeEpoch,
    pub(crate) drained_probe_epoch: Option<quiescence::OutputProbeEpoch>,
}

#[derive(Debug)]
pub(crate) struct PendingOutput {
    pub(crate) data: Vec<u8>,
    pub(crate) attempt: Option<crate::journal::AppendAttempt>,
}

#[derive(Debug, Default)]
pub(crate) struct ReaderLifecycle {
    pub(crate) progress: Mutex<ReaderProgress>,
    pub(crate) changed: Condvar,
    pub(crate) cancelled: AtomicBool,
    pub(crate) output_reader_wakeup: AtomicBool,
    #[cfg(test)]
    restart_wait_probe: Mutex<Option<std::sync::mpsc::SyncSender<()>>>,
    #[cfg(test)]
    pub(crate) child_wait_publication_after_lock: super::control_test_hooks::BlockingControlHook,
}

include!("reader_lifecycle_state.rs");

#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../test_support/unit/reader/reader_lifecycle_test_support.rs"]
mod test_support;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/reader/reader_lifecycle_tests.rs"]
mod tests;

#[path = "reader_lifecycle_quiescence.rs"]
mod quiescence;
pub(crate) use quiescence::OutputProbeEpoch;
#[path = "reader_lifecycle_eof_state.rs"]
mod eof_state;
pub(crate) use eof_state::KernelEofTransactionDisposition;
#[path = "reader_lifecycle_kernel_read.rs"]
mod kernel_read;
pub(crate) use kernel_read::{KernelPtyEofRead, PtyReadOutcome};
#[path = "reader_lifecycle_exit_wait.rs"]
mod exit_wait;
#[cfg(test)]
pub(crate) use exit_wait::ReaderExitConfirmation;
#[path = "reader_lifecycle_child_wait.rs"]
mod child_wait;
#[path = "reader_lifecycle_completion.rs"]
mod completion;
#[path = "reader_supervisor_completion.rs"]
mod supervisor_completion;
pub(crate) use supervisor_completion::ReaderSupervisorCustody;
#[path = "reader_lifecycle_missing_eof.rs"]
mod missing_eof;
#[path = "reader_lifecycle_namespace_loss.rs"]
pub(crate) mod namespace_loss;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/reader/reader_lifecycle_namespace_loss_tests.rs"]
mod namespace_loss_tests;
