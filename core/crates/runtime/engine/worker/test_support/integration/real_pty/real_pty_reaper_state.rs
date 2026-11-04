const FAULT_REAPER_COMPLETION_TIMEOUT_DETAIL: &str =
    "deferred PTY fault reaper did not publish completion before the deadline";
const FAULT_REAPER_TERMINAL_STATE_DETAIL: &str =
    "deferred PTY fault reaper published an invalid terminal state";
const ROOT_EXIT_CLEANUP_START_TIMEOUT_DETAIL: &str =
    "PTY owner did not choose an inline or deferred cleanup path before the deadline";
const ROOT_EXIT_CLEANUP_STATE_DETAIL: &str =
    "PTY owner published an invalid root-exit cleanup state";
const INLINE_REAPER_STATE_DETAIL: &str =
    "inline root-exit cleanup unexpectedly transferred custody to the fault reaper";
const FAULT_REAP_RETAINED_SAMPLE_COUNT: usize = 3;
const FAULT_REAP_STATUS_MISSING_DETAIL: &str =
    "deferred PTY fault-reap status is missing from the runtime view";
const FAULT_REAP_PROGRESS_MISSING_DETAIL: &str =
    "deferred PTY fault-reap semantic progress is missing";
const FAULT_REAP_CUSTODY_LOST_DETAIL: &str =
    "deferred PTY fault reaper lost custody before the remote writer was released";
const FAULT_REAP_SEQUENCE_REGRESSED_DETAIL: &str =
    "fault-reap semantic progress sequence regressed";
const FAULT_REAP_WIRE_MISMATCH_DETAIL: &str =
    "fault-reap runtime status changed across its Cap'n Proto wire round-trip";
pub(super) const FAULT_REAP_DEFERRED_PATH_DETAIL: &str =
    "macOS signal-resistant PTY cleanup did not retain deferred custody";
const FAULT_REAP_TERMINAL_CUSTODY_DETAIL: &str =
    "completed PTY fault reaper still reports retained custody";
const FAULT_REAP_TERMINAL_PROGRESS_DETAIL: &str =
    "completed PTY fault reaper did not publish its terminal progress";

impl GatedOwner {
    pub(super) fn release_after_cleanup_started(&mut self, timeout: Duration) -> RootExitCleanupPath {
        self.release();
        let deadline = Instant::now() + timeout;
        loop {
            let settlement = self.settlement.get();
            let latest = self.reaper_state.latest();
            if matches!(settlement, Some(Ok(()))) {
                assert!(
                    latest.is_none(),
                    "{INLINE_REAPER_STATE_DETAIL}: {latest:#?}"
                );
                return RootExitCleanupPath::InlineComplete;
            }
            if matches!(
                settlement,
                Some(Err(
                    pty_owner_handle::PtyOwnerCommandFailure::CleanupDeferred(_)
                ))
            ) && latest.is_some()
            {
                return RootExitCleanupPath::FaultReaperDeferred;
            }
            assert!(
                matches!(
                    settlement,
                    None | Some(Err(
                        pty_owner_handle::PtyOwnerCommandFailure::CleanupDeferred(_)
                    ))
                ),
                "{ROOT_EXIT_CLEANUP_STATE_DETAIL}; settlement: {settlement:?}; \
                 latest state: {latest:#?}"
            );
            assert!(
                Instant::now() < deadline,
                "{ROOT_EXIT_CLEANUP_START_TIMEOUT_DETAIL}; settlement: {settlement:?}; \
                 latest state: {latest:#?}"
            );
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }

    pub(super) fn wait_for_cleanup_completion(&self, path: RootExitCleanupPath, timeout: Duration) {
        if path == RootExitCleanupPath::InlineComplete {
            return;
        }
        let deadline = Instant::now() + timeout;
        loop {
            let latest = self.reaper_state.latest();
            if let Some(snapshot) = latest.as_ref()
                && snapshot.completion_published
            {
                assert_fault_reap_terminal(snapshot, self.reaper_state.status());
                return;
            }
            assert!(
                Instant::now() < deadline,
                "{FAULT_REAPER_COMPLETION_TIMEOUT_DETAIL}; latest state: {latest:#?}; \
                 reader cancelled: {}; reader progress: {:#?}",
                self.reader_lifecycle.is_cancelled(),
                lock_recover(&self.reader_lifecycle.progress),
            );
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }

    pub(super) fn assert_retained_runtime_status(&self, state: &WorkerState, path: RootExitCleanupPath) {
        if path == RootExitCleanupPath::InlineComplete {
            return;
        }
        let mut previous_sequence = None;
        for _ in 0..FAULT_REAP_RETAINED_SAMPLE_COUNT {
            let status = runtime_fault_reap_status(state);
            assert!(status.custody_retained, "{FAULT_REAP_CUSTODY_LOST_DETAIL}");
            let progress = status
                .last_semantic_progress
                .expect(FAULT_REAP_PROGRESS_MISSING_DETAIL);
            if let Some(previous) = previous_sequence {
                assert!(
                    progress.sequence >= previous,
                    "{FAULT_REAP_SEQUENCE_REGRESSED_DETAIL}: {previous} -> {}",
                    progress.sequence
                );
            }
            previous_sequence = Some(progress.sequence);
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }
}

fn assert_fault_reap_terminal(
    snapshot: &pty_fault_reaper::FaultReapStateSnapshot,
    status: Option<crate::model::FaultReapStatus>,
) {
    assert!(
        snapshot.custody.is_none()
            && !snapshot.wait_authority_present
            && !snapshot.wait_authority_lost
            && !snapshot.root_exit_proof_present
            && !snapshot.root_gone_without_wait
            && !snapshot.observer_only
            && snapshot.completion_published
            && !snapshot.failure_present
            && snapshot.output_fence_reached != Some(false),
        "{FAULT_REAPER_TERMINAL_STATE_DETAIL}: {snapshot:#?}; status: {status:#?}"
    );
}

fn runtime_fault_reap_status(state: &WorkerState) -> crate::model::FaultReapStatus {
    let runtime = state.status();
    let encoded = yolo_capnp_codec::encode(&runtime).unwrap();
    let decoded: crate::model::RuntimeView = yolo_capnp_codec::decode(&encoded).unwrap();
    assert_eq!(
        decoded.fault_reap, runtime.fault_reap,
        "{FAULT_REAP_WIRE_MISMATCH_DETAIL}"
    );
    decoded.fault_reap.expect(FAULT_REAP_STATUS_MISSING_DETAIL)
}

pub(super) fn assert_fault_reap_runtime_completed(state: &WorkerState, path: RootExitCleanupPath) {
    if path == RootExitCleanupPath::InlineComplete {
        return;
    }
    let status = runtime_fault_reap_status(state);
    assert!(
        !status.custody_retained,
        "{FAULT_REAP_TERMINAL_CUSTODY_DETAIL}"
    );
    assert_eq!(
        status
            .last_semantic_progress
            .expect(FAULT_REAP_PROGRESS_MISSING_DETAIL)
            .kind,
        crate::model::FaultReapProgressKind::Completed,
        "{FAULT_REAP_TERMINAL_PROGRESS_DETAIL}"
    );
}
