const FIXTURE_TERMINATION_CHANNEL_CAPACITY: usize = 1;
const FIXTURE_TERMINATION_THREAD: &str = "yolo-fixture-termination";
const FIXTURE_TERMINATION_TIMEOUT: Duration =
    WORKER_TERMINATION_TIMEOUT.saturating_add(FIXTURE_HOST_RELEASE_COMPLETION_TIMEOUT);
const FIXTURE_TERMINATION_THREAD_DETAIL: &str = "termination fixture cleanup thread failed";
const FIXTURE_TERMINATION_COMPLETION_DETAIL: &str = "termination fixture cleanup did not complete";

type FixtureTerminationResult = std::result::Result<(), String>;
type FixtureTerminationObservation = Arc<OnceLock<FixtureTerminationResult>>;

struct FixtureTerminationRuntime {
    executor: tokio::runtime::Handle,
    request: Option<tokio::sync::oneshot::Sender<FixtureTerminationWork>>,
    complete: Option<std::sync::mpsc::Receiver<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    observation: FixtureTerminationObservation,
    release: Option<FixtureHostRelease>,
    settlement: Option<pty_owner_handle::OwnerSettlement>,
}

struct FixtureTerminationWork {
    release: Option<FixtureHostRelease>,
    settlement: Option<pty_owner_handle::OwnerSettlement>,
}

impl FixtureTerminationRuntime {
    fn prepare(state: Arc<WorkerState>) -> Self {
        let root = state.pty_root_identity();
        let (ready, executor) = std::sync::mpsc::sync_channel(FIXTURE_TERMINATION_CHANNEL_CAPACITY);
        let (completed, complete) =
            std::sync::mpsc::sync_channel(FIXTURE_TERMINATION_CHANNEL_CAPACITY);
        let (request, commands) = tokio::sync::oneshot::channel::<FixtureTerminationWork>();
        let observation = Arc::new(OnceLock::new());
        let observed = Arc::clone(&observation);
        let thread = std::thread::Builder::new()
            .name(FIXTURE_TERMINATION_THREAD.into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect(FIXTURE_TERMINATION_THREAD_DETAIL);
                ready
                    .send(runtime.handle().clone())
                    .expect(FIXTURE_TERMINATION_THREAD_DETAIL);
                // A caller deadline must not cancel a thread that still owns PTY cleanup.
                let result = runtime.block_on(async move {
                    let work = commands
                        .await
                        .context(FIXTURE_TERMINATION_COMPLETION_DETAIL)?;
                    work.settle(state, root).await
                });
                let _ = observed.set(result.map_err(|error| format!("{error:#}")));
                runtime.shutdown_timeout(FIXTURE_HOST_RELEASE_COMPLETION_TIMEOUT);
                let _ = completed.send(());
            })
            .expect(FIXTURE_TERMINATION_THREAD_DETAIL);
        let executor = executor
            .recv_timeout(FIXTURE_HOST_RELEASE_COMPLETION_TIMEOUT)
            .expect(FIXTURE_TERMINATION_THREAD_DETAIL);
        Self {
            executor,
            request: Some(request),
            complete: Some(complete),
            thread: Some(thread),
            observation,
            release: None,
            settlement: None,
        }
    }

    fn submit(&mut self) {
        if let Some(request) = self.request.take() {
            let _ = request.send(FixtureTerminationWork {
                release: self.release.take(),
                settlement: self.settlement.take(),
            });
        }
    }

    async fn finish(mut self) {
        self.submit();
        let complete = self.complete.take().unwrap();
        let thread = self.thread.take().unwrap();
        tokio::task::spawn_blocking(move || join_fixture_termination(complete, thread))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(self.observation.get(), Some(&Ok(())));
    }
}

impl Drop for FixtureTerminationRuntime {
    fn drop(&mut self) {
        self.submit();
        let Some(complete) = self.complete.take() else {
            return;
        };
        let Some(thread) = self.thread.take() else {
            return;
        };
        let result = join_fixture_termination(complete, thread).and_then(|()| {
            self.observation
                .get()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!(FIXTURE_TERMINATION_COMPLETION_DETAIL))?
                .map_err(anyhow::Error::msg)
        });
        if let Err(error) = result {
            if std::thread::panicking() {
                eprintln!("{FIXTURE_TERMINATION_COMPLETION_DETAIL}: {error:#}");
            } else {
                panic!("{FIXTURE_TERMINATION_COMPLETION_DETAIL}: {error:#}");
            }
        }
    }
}

fn join_fixture_termination(
    complete: std::sync::mpsc::Receiver<()>,
    thread: std::thread::JoinHandle<()>,
) -> Result<()> {
    complete
        .recv_timeout(
            FIXTURE_TERMINATION_TIMEOUT.saturating_add(FIXTURE_HOST_RELEASE_COMPLETION_TIMEOUT),
        )
        .context(FIXTURE_TERMINATION_COMPLETION_DETAIL)?;
    thread
        .join()
        .map_err(|_| anyhow::anyhow!(FIXTURE_TERMINATION_THREAD_DETAIL))
}
