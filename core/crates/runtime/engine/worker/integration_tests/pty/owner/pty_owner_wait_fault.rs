use crate::worker::tests::*;
const WAIT_FAULT_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_FAULT_SHELL: &str = "/bin/sh";
const WAIT_FAULT_COMMAND_ARGUMENT: &str = "-c";
const WAIT_FAULT_COMMAND: &str = "read gate; exit 0";
const WAIT_FAULT_TRIGGER: &[u8] = b"exit\n";
const EXPECTED_PORTABLE_WAIT_CALLBACKS: usize = 0;

#[derive(Debug)]
struct PanickingWaitChild {
    inner: Box<dyn portable_pty::Child + Send + Sync>,
    wait_callbacks: Arc<WaitFaultCounter>,
}

impl portable_pty::ChildKiller for PanickingWaitChild {
    fn kill(&mut self) -> std::io::Result<()> {
        self.inner.kill()
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        self.inner.clone_killer()
    }
}

impl portable_pty::Child for PanickingWaitChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        self.wait_callbacks.fetch_add(1, Ordering::AcqRel);
        panic!("portable child try_wait must not own PTY reaping")
    }

    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        self.wait_callbacks.fetch_add(1, Ordering::AcqRel);
        panic!("portable child wait must not own PTY reaping")
    }

    fn process_id(&self) -> Option<u32> {
        self.inner.process_id()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn natural_exit_uses_raw_reap_without_portable_wait_callbacks() {
    let mut command = CommandBuilder::new(WAIT_FAULT_SHELL);
    command.arg(WAIT_FAULT_COMMAND_ARGUMENT);
    command.arg(WAIT_FAULT_COMMAND);
    let fixture = Fixture::with_command(command);
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let child = fixture.child.lock().unwrap().take().unwrap();
    let wait_callbacks = Arc::new(WaitFaultCounter::new(0));
    start_wait_fault_owner(&fixture, child, Arc::clone(&wait_callbacks));
    lock_recover(&fixture.state.writer)
        .as_mut()
        .unwrap()
        .write_all(WAIT_FAULT_TRIGGER)
        .unwrap();

    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, WAIT_FAULT_TIMEOUT).await;

    assert_eq!(
        lock_recover(&fixture.state.exit).as_ref().unwrap().reason,
        ExitReason::Natural
    );
    assert_eq!(
        wait_callbacks.load(Ordering::Acquire),
        EXPECTED_PORTABLE_WAIT_CALLBACKS
    );
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
}

fn start_wait_fault_owner(
    fixture: &Fixture,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    wait_callbacks: Arc<WaitFaultCounter>,
) {
    let record = lock_recover(&fixture.state.record);
    let root = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    drop(record);
    let child = Box::new(PanickingWaitChild {
        inner: child,
        wait_callbacks,
    });
    let mut guard =
        StartupChildGuard::with_reader_backed_output(child, fixture_runtime_output(fixture));
    guard.track(root).unwrap();
    pty_owner_handle::spawn_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
}
use std::sync::atomic::AtomicUsize as WaitFaultCounter;
