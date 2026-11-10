use crate::worker::tests::*;
const HOOK_PUMP_TEST_HOST_SOURCE: &str = "/bin/echo";
const HOOK_PUMP_TEST_INPUT: &[u8] = b"claude\n";
const HOOK_PUMP_EXPECTED_WRITES: usize = 1;

#[derive(Clone, Default)]
struct HookPumpWriteCapture {
    bytes: Arc<Mutex<Vec<u8>>>,
    writes: Arc<std::sync::atomic::AtomicUsize>,
}

impl std::io::Write for HookPumpWriteCapture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.writes.fetch_add(1, Ordering::AcqRel);
        self.bytes.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn bootstrap_input_waits_until_hook_listener_is_armed_and_released() {
    let fixture = Fixture::new();
    let server = hook_pump_server(&fixture).await;
    let shell = hook_pump_shell(&fixture);
    let mut tasks = JoinSet::new();
    let (armed, armed_observed) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let mut pump = claude_hook::spawn_admission_pump_after_test_barrier(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        shell,
        armed,
        released,
    );
    let capture = HookPumpWriteCapture::default();
    let mut writer = PtyInputWriter::test(capture.clone());
    let mut delivery = Box::pin(runtime_bootstrap_input::deliver_ready_input(
        &mut tasks,
        &mut pump,
        &mut writer,
        Some(HOOK_PUMP_TEST_INPUT),
    ));
    hook_pump_assert_send(&delivery);

    assert!(hook_pump_poll_once(delivery.as_mut()).await.is_pending());
    armed_observed.await.unwrap();
    assert_eq!(capture.writes.load(Ordering::Acquire), 0);
    assert!(capture.bytes.lock().unwrap().is_empty());

    release.send(()).unwrap();
    let attempt = delivery.await.unwrap();
    pump.activate().unwrap();
    assert_eq!(attempt.delivery, BootstrapDelivery::Delivered);
    assert_eq!(
        capture.writes.load(Ordering::Acquire),
        HOOK_PUMP_EXPECTED_WRITES
    );
    assert_eq!(
        capture.bytes.lock().unwrap().as_slice(),
        HOOK_PUMP_TEST_INPUT
    );
    abort_and_drain_tasks(&mut tasks).await;
}

#[tokio::test(flavor = "current_thread")]
async fn hook_pump_barrier_failure_never_writes_bootstrap_input() {
    let fixture = Fixture::new();
    let server = hook_pump_server(&fixture).await;
    let shell = hook_pump_shell(&fixture);
    let mut tasks = JoinSet::new();
    let (armed, armed_observed) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let mut pump = claude_hook::spawn_admission_pump_after_test_barrier(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        shell,
        armed,
        released,
    );
    let capture = HookPumpWriteCapture::default();
    let mut writer = PtyInputWriter::test(capture.clone());
    let delivery = runtime_bootstrap_input::deliver_ready_input(
        &mut tasks,
        &mut pump,
        &mut writer,
        Some(HOOK_PUMP_TEST_INPUT),
    );

    armed_observed.await.unwrap();
    drop(release);
    assert!(delivery.await.is_err());
    assert_eq!(capture.writes.load(Ordering::Acquire), 0);
    assert!(capture.bytes.lock().unwrap().is_empty());
    abort_and_drain_tasks(&mut tasks).await;
}

#[tokio::test(flavor = "current_thread")]
async fn bootstrap_without_input_does_not_wait_for_hook_readiness() {
    let fixture = Fixture::new();
    let server = hook_pump_server(&fixture).await;
    let shell = hook_pump_shell(&fixture);
    let mut tasks = JoinSet::new();
    let (armed, armed_observed) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let mut pump = claude_hook::spawn_admission_pump_after_test_barrier(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        shell,
        armed,
        released,
    );
    let capture = HookPumpWriteCapture::default();
    let mut writer = PtyInputWriter::test(capture.clone());

    let attempt =
        runtime_bootstrap_input::deliver_ready_input(&mut tasks, &mut pump, &mut writer, None)
            .await
            .unwrap();
    assert_eq!(attempt.delivery, BootstrapDelivery::NotRequired);
    assert_eq!(capture.writes.load(Ordering::Acquire), 0);
    assert!(capture.bytes.lock().unwrap().is_empty());

    armed_observed.await.unwrap();
    release.send(()).unwrap();
    pump.activate().unwrap();
    assert!(tasks.join_next().await.unwrap().unwrap().is_none());
    {
        let mut pump = Box::pin(tasks.join_next());
        assert!(hook_pump_poll_once(pump.as_mut()).await.is_pending());
    }
    abort_and_drain_tasks(&mut tasks).await;
}

pub(super) async fn hook_pump_server(fixture: &Fixture) -> ClaudeHookServer {
    let settings = fixture
        .state
        .paths
        .session_dir(fixture.terminal_id())
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    ClaudeHookServer::bind_test_source(
        &fixture.state.namespace,
        &fixture.state.paths,
        fixture.terminal_id(),
        settings,
        std::path::Path::new(HOOK_PUMP_TEST_HOST_SOURCE),
    )
    .await
    .unwrap()
}

pub(super) fn hook_pump_shell(fixture: &Fixture) -> process::ProcessIdentity {
    let record = lock_recover(&fixture.state.record);
    process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    }
}

pub(super) async fn hook_pump_poll_once<F: std::future::Future>(
    mut future: std::pin::Pin<&mut F>,
) -> std::task::Poll<F::Output> {
    std::future::poll_fn(|context| std::task::Poll::Ready(future.as_mut().poll(context))).await
}

fn hook_pump_assert_send<T: Send>(_: &T) {}
