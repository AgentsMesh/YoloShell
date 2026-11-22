const BOUNDED_TEST_TASK_HANDLE_MISSING_DETAIL: &str = "controlled test task handle is missing";
const BOUNDED_TEST_TASK_JOIN_FAILURE_DETAIL: &str = "controlled test task failed while joining";
const BOUNDED_TEST_TASK_COMPLETION_TIMEOUT_DETAIL: &str =
    "controlled test task exceeded its deadline";
const BOUNDED_TEST_TASK_ABORT_TIMEOUT_DETAIL: &str =
    "controlled test task did not abort before its deadline";
const EXITED_TERMINATION_TASK_LABEL: &str = "exited-at-entry termination";
const TERMINATION_TOMBSTONE_TIMEOUT_DETAIL: &str =
    "termination did not reach its persisted tombstone barrier before the deadline";

struct BoundedTestTask<T> {
    label: &'static str,
    task: Option<tokio::task::JoinHandle<T>>,
}

impl<T: Send + 'static> BoundedTestTask<T> {
    fn spawn(label: &'static str, future: impl Future<Output = T> + Send + 'static) -> Self {
        Self {
            label,
            task: Some(tokio::spawn(future)),
        }
    }

    async fn finish(mut self) -> T {
        let mut task = self
            .task
            .take()
            .expect(BOUNDED_TEST_TASK_HANDLE_MISSING_DETAIL);
        match tokio::time::timeout(CONTROL_BARRIER_TIMEOUT, &mut task).await {
            Ok(joined) => joined.unwrap_or_else(|error| {
                panic!(
                    "{}: {BOUNDED_TEST_TASK_JOIN_FAILURE_DETAIL}: {error}",
                    self.label
                )
            }),
            Err(_) => {
                task.abort();
                let _ = tokio::time::timeout(CONTROL_BARRIER_TIMEOUT, &mut task)
                    .await
                    .unwrap_or_else(|_| {
                        panic!("{}: {BOUNDED_TEST_TASK_ABORT_TIMEOUT_DETAIL}", self.label)
                    });
                panic!(
                    "{}: {BOUNDED_TEST_TASK_COMPLETION_TIMEOUT_DETAIL}",
                    self.label
                );
            }
        }
    }
}

impl<T> Drop for BoundedTestTask<T> {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

type TerminationDispatchResult = std::result::Result<ResponseEnvelope, ErrorBody>;

struct ControlledTerminationTask {
    state: Arc<WorkerState>,
    task: Option<BoundedTestTask<TerminationDispatchResult>>,
    released: bool,
}

impl ControlledTerminationTask {
    fn spawn(state: Arc<WorkerState>, request: FencedTerminateRequest) -> Self {
        let dispatch_state = Arc::clone(&state);
        let task = BoundedTestTask::spawn(EXITED_TERMINATION_TASK_LABEL, async move {
            dispatch_test(&dispatch_state, method::TERMINATE_RUNTIME_EXACT, &request).await
        });
        Self {
            state,
            task: Some(task),
            released: false,
        }
    }

    async fn wait_until_tombstone(mut self) -> Self {
        let reached = tokio::time::timeout(
            CONTROL_BARRIER_TIMEOUT,
            self.state
                .control_test_hooks
                .termination_after_tombstone
                .wait_until_reached(),
        )
        .await;
        if reached.is_err() {
            self.resume_and_reap().await;
            panic!("{TERMINATION_TOMBSTONE_TIMEOUT_DETAIL}");
        }
        self
    }

    async fn finish(mut self) -> TerminationDispatchResult {
        self.resume();
        self.task
            .take()
            .expect(BOUNDED_TEST_TASK_HANDLE_MISSING_DETAIL)
            .finish()
            .await
    }

    async fn resume_and_reap(&mut self) {
        self.resume();
        if let Some(task) = self.task.take() {
            let _ = task.finish().await;
        }
    }

    fn resume(&mut self) {
        if !self.released {
            self.state
                .control_test_hooks
                .termination_after_tombstone
                .resume();
            self.released = true;
        }
    }
}

impl Drop for ControlledTerminationTask {
    fn drop(&mut self) {
        self.resume();
    }
}
