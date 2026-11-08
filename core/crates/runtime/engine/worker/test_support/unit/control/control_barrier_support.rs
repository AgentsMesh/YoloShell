const CONTROL_BARRIER_TIMEOUT: Duration = Duration::from_secs(2);
const BARRIER_TEST_COLS: u16 = 111;
const BARRIER_TEST_ROWS: u16 = 37;
const BARRIER_TEST_SIGNAL: &str = "CONT";
const TERMINATION_GATE_RELEASE_DETAIL: &str =
    "termination owner wait retained the mutation gate after command submission";

struct BlockingInputWriter {
    reached: Option<mpsc::SyncSender<()>>,
    release: mpsc::Receiver<()>,
    writes: Arc<std::sync::atomic::AtomicUsize>,
}

impl Write for BlockingInputWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.writes.fetch_add(1, Ordering::AcqRel);
        if let Some(reached) = self.reached.take() {
            reached.send(()).map_err(|_| io::ErrorKind::BrokenPipe)?;
            self.release.recv().map_err(|_| io::ErrorKind::BrokenPipe)?;
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn install_blocking_writer(
    state: &WorkerState,
) -> (
    mpsc::Receiver<()>,
    mpsc::SyncSender<()>,
    Arc<std::sync::atomic::AtomicUsize>,
) {
    let (reached_tx, reached_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *lock_recover(&state.writer) = Some(PtyInputWriter::test(BlockingInputWriter {
        reached: Some(reached_tx),
        release: release_rx,
        writes: Arc::clone(&writes),
    }));
    (reached_rx, release_tx, writes)
}

async fn wait_for_sync_signal(receiver: mpsc::Receiver<()>) {
    tokio::task::spawn_blocking(move || receiver.recv_timeout(CONTROL_BARRIER_TIMEOUT))
        .await
        .unwrap()
        .unwrap();
}

async fn assert_task_pending<T>(task: &tokio::task::JoinHandle<T>) {
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
}

struct BlockingResizeMaster {
    reached: Mutex<Option<mpsc::SyncSender<()>>>,
    release: Mutex<mpsc::Receiver<()>>,
    sizes: Arc<Mutex<Vec<PtySize>>>,
}

impl MasterPty for BlockingResizeMaster {
    fn resize(&self, size: PtySize) -> Result<()> {
        if let Some(reached) = lock_recover(&self.reached).take() {
            reached.send(()).context("report blocked test resize")?;
            lock_recover(&self.release)
                .recv()
                .context("release blocked test resize")?;
        }
        lock_recover(&self.sizes).push(size);
        Ok(())
    }

    fn get_size(&self) -> Result<PtySize> {
        Ok(lock_recover(&self.sizes)
            .last()
            .copied()
            .unwrap_or_default())
    }

    fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>> {
        bail!("blocking resize master has no reader")
    }

    fn take_writer(&self) -> Result<Box<dyn Write + Send>> {
        bail!("blocking resize master has no writer")
    }

    #[cfg(unix)]
    fn process_group_leader(&self) -> Option<libc::pid_t> {
        None
    }

    #[cfg(unix)]
    fn as_raw_fd(&self) -> Option<std::os::fd::RawFd> {
        None
    }

    #[cfg(unix)]
    fn tty_name(&self) -> Option<PathBuf> {
        None
    }
}

fn install_blocking_master(
    state: &WorkerState,
) -> (
    mpsc::Receiver<()>,
    mpsc::SyncSender<()>,
    Arc<Mutex<Vec<PtySize>>>,
) {
    let (reached_tx, reached_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let sizes = Arc::new(Mutex::new(Vec::new()));
    *lock_recover(&state.master) = Some(Box::new(BlockingResizeMaster {
        reached: Mutex::new(Some(reached_tx)),
        release: Mutex::new(release_rx),
        sizes: Arc::clone(&sizes),
    }));
    (reached_rx, release_tx, sizes)
}

fn barrier_input(fixture: &Fixture, lease: &WriterLeaseGrant) -> InputRequest {
    InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: b"barrier-input".to_vec(),
    }
}
