const FINALIZER_COMPLETION_CHANNEL_CAPACITY: usize = 1;
const FINALIZER_EXIT_CODE: i32 = 0;
const FINALIZER_LEASE_ID: &str = "finalizer";
const FINALIZER_INPUT_TASK_LABEL: &str = "finalizer admitted input";
const FINALIZER_COMPLETION_TIMEOUT_DETAIL: &str = "finalizer did not complete before its deadline";
const FINALIZER_ADMISSION_TIMEOUT_DETAIL: &str =
    "finalizer did not reach mutation admission before its deadline";
const FINALIZER_THREAD_PANIC_DETAIL: &str = "finalizer thread panicked";
const FINALIZER_HOOK_TIMEOUT_DETAIL: &str =
    "finalizer did not reach the shutdown hook before its deadline";
const FINALIZER_EVENT_TIMEOUT_DETAIL: &str =
    "finalizer did not publish the lease event before its deadline";
const FINALIZER_ADOPTION_TIMEOUT_DETAIL: &str =
    "controller adoption did not resolve before its deadline";

enum FinalizerRelease {
    Writer(mpsc::SyncSender<()>),
    ShutdownHook,
}

#[derive(Clone, Copy)]
enum FinalizerCleanupPolicy {
    ReportFailures,
    SuppressFailures,
}

struct FinalizerThread {
    state: Arc<WorkerState>,
    release: Option<FinalizerRelease>,
    completion: Option<mpsc::Receiver<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl FinalizerThread {
    fn spawn(state: Arc<WorkerState>, release: FinalizerRelease) -> Self {
        Self::spawn_delayed(state, release, Duration::ZERO)
    }

    fn spawn_delayed(state: Arc<WorkerState>, release: FinalizerRelease, delay: Duration) -> Self {
        let (completion_tx, completion_rx) =
            mpsc::sync_channel(FINALIZER_COMPLETION_CHANNEL_CAPACITY);
        let finalizer_state = Arc::clone(&state);
        let thread = std::thread::spawn(move || {
            std::thread::sleep(delay);
            finalize_child_exit(
                &finalizer_state,
                Some(FINALIZER_EXIT_CODE),
                None,
                Duration::ZERO,
            );
            let _ = completion_tx.send(());
        });
        Self {
            state,
            release: Some(release),
            completion: Some(completion_rx),
            thread: Some(thread),
        }
    }

    async fn wait_until_system_admission(&mut self) {
        let reached = tokio::time::timeout(
            CONTROL_BARRIER_TIMEOUT,
            self.state
                .control_test_hooks
                .system_mutation_before_gate
                .wait_until_reached(),
        )
        .await;
        if reached.is_err() {
            self.finish_inner(FinalizerCleanupPolicy::ReportFailures);
            panic!("{FINALIZER_ADMISSION_TIMEOUT_DETAIL}");
        }
    }

    fn release_gate(&mut self) {
        let Some(release) = self.release.take() else {
            return;
        };
        match release {
            FinalizerRelease::Writer(sender) => {
                let _ = sender.send(());
            }
            FinalizerRelease::ShutdownHook => self
                .state
                .control_test_hooks
                .finalizer_before_shutdown
                .resume(),
        }
    }

    async fn wait_until_shutdown_hook(&mut self) {
        // The isolated process bounds the whole scenario, including durable finalization.
        let reached = tokio::time::timeout(FINALIZER_PROCESS_TIMEOUT, async {
            while !self
                .state
                .control_test_hooks
                .finalizer_before_shutdown
                .is_reached()
            {
                tokio::task::yield_now().await;
            }
        })
        .await;
        if reached.is_err() {
            self.finish_inner(FinalizerCleanupPolicy::ReportFailures);
            panic!("{FINALIZER_HOOK_TIMEOUT_DETAIL}");
        }
    }

    fn finish(mut self) {
        self.finish_inner(FinalizerCleanupPolicy::ReportFailures);
    }

    fn finish_inner(&mut self, policy: FinalizerCleanupPolicy) {
        self.release_gate();
        let Some(completion) = self.completion.as_ref() else {
            return;
        };
        let completion_timed_out = matches!(
            completion.recv_timeout(CONTROL_BARRIER_TIMEOUT),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        self.completion.take();
        // Rust threads cannot be cancelled. The controlled gate is already
        // released, so joining here is the only non-detaching cleanup path.
        let joined = self.thread.take().map(std::thread::JoinHandle::join);
        let report_failures = matches!(policy, FinalizerCleanupPolicy::ReportFailures);
        if report_failures && joined.is_some_and(|result| result.is_err()) {
            panic!("{FINALIZER_THREAD_PANIC_DETAIL}");
        }
        if report_failures && completion_timed_out {
            panic!("{FINALIZER_COMPLETION_TIMEOUT_DETAIL}");
        }
    }
}

impl Drop for FinalizerThread {
    fn drop(&mut self) {
        self.finish_inner(FinalizerCleanupPolicy::SuppressFailures);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControllerCustodySnapshot {
    active_generation: u64,
    persisted_generation: u64,
    record_generation: u64,
    authority: Option<secondary_custody_authority::ReadyWorkerCustodyIdentity>,
}

impl ControllerCustodySnapshot {
    fn capture(fixture: &Fixture) -> Self {
        Self {
            active_generation: fixture.state.controller_generation.load(Ordering::Acquire),
            persisted_generation: fixture
                .state
                .controller_persisted_generation
                .load(Ordering::Acquire),
            record_generation: lock_recover(&fixture.state.record).controller_generation,
            authority: fixture.state.secondary_custody_authority.active_for_test(),
        }
    }

    fn assert_termination_intent_rejection(self, after: Self, adoption: ErrorBody) {
        assert_eq!(adoption.code, crate::protocol::error_code::CONFLICT);
        assert_eq!(
            adoption.message,
            secondary_custody_promotion::SUCCESSOR_TERMINATION_INTENT_DETAIL
        );
        assert_eq!(after, self);
    }
}
