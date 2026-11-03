const REMOVAL_POLLING_TIMEOUT: Duration = Duration::from_secs(5);
const REMOVAL_POLLING_PANIC: &str = "injected removal observer panic";

struct RemovalPollingProbe {
    calls: AtomicU64,
    started: tokio::sync::mpsc::UnboundedSender<()>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

impl RemovalPollingProbe {
    fn observe(&self) -> SessionRemovalObservation {
        self.calls.fetch_add(1, Ordering::AcqRel);
        self.started.send(()).unwrap();
        lock_recover(&self.release)
            .recv_timeout(REMOVAL_POLLING_TIMEOUT)
            .unwrap();
        SessionRemovalObservation::Present
    }
}

fn removal_polling_probe() -> (
    Arc<RemovalPollingProbe>,
    tokio::sync::mpsc::UnboundedReceiver<()>,
    std::sync::mpsc::Sender<()>,
) {
    let (started, receiver) = tokio::sync::mpsc::unbounded_channel();
    let (release, released) = std::sync::mpsc::channel();
    (
        Arc::new(RemovalPollingProbe {
            calls: AtomicU64::new(0),
            started,
            release: Mutex::new(released),
        }),
        receiver,
        release,
    )
}

fn poll_removal_once<F: Future>(future: std::pin::Pin<&mut F>) -> std::task::Poll<F::Output> {
    future.poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
}
