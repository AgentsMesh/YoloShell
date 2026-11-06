use crate::worker::tests::*;
const SUPERVISION_TEST_TIMEOUT: Duration = Duration::from_secs(2);
const EXIT_FINALIZATION_TEST_TIMEOUT: Duration = Duration::from_secs(5);
const SUPERVISION_TEST_POLL: Duration = Duration::from_millis(10);
const WAITER_RESULT_TIMEOUT_DETAIL: &str = "child wait retry phase did not resolve";
const WAITER_STATUS_MISSING_DETAIL: &str = "live child retry must resolve an exit status";
const READER_RECOVERY_INPUT: &[u8] = b"reader-recovered\n";
const FLAKY_WAIT_EXIT_CODE: u32 = 7;
pub(super) const UNUSED_SHELL_PID: u32 = 0;

#[derive(Debug)]
struct FlakyWaitChild {
    attempts: Arc<std::sync::atomic::AtomicUsize>,
}

impl portable_pty::ChildKiller for FlakyWaitChild {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self {
            attempts: Arc::clone(&self.attempts),
        })
    }
}

impl portable_pty::Child for FlakyWaitChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        Ok(None)
    }

    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        match self.attempts.fetch_add(1, Ordering::AcqRel) {
            0 => panic!("injected wait panic"),
            1 => Err(std::io::ErrorKind::BrokenPipe.into()),
            _ => Ok(portable_pty::ExitStatus::with_exit_code(
                FLAKY_WAIT_EXIT_CODE,
            )),
        }
    }

    fn process_id(&self) -> Option<u32> {
        Some(std::process::id())
    }
}

#[derive(Debug)]
struct MissingWaitChild;

impl portable_pty::ChildKiller for MissingWaitChild {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self)
    }
}

impl portable_pty::Child for MissingWaitChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }

    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }

    fn process_id(&self) -> Option<u32> {
        Some(u32::MAX)
    }
}

pub(super) async fn wait_for_exit(state: &WorkerState) -> ExitRecord {
    tokio::time::timeout(EXIT_FINALIZATION_TEST_TIMEOUT, async {
        loop {
            if let Some(exit) = state.exit.lock().unwrap().clone() {
                return exit;
            }
            tokio::time::sleep(SUPERVISION_TEST_POLL).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn reader_error_reclones_master_without_terminating_the_child() {
    let fixture = Fixture::new();
    spawn_reader(Box::new(FailingIo), Arc::clone(&fixture.state)).unwrap();
    fixture
        .state
        .writer
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .write_all(READER_RECOVERY_INPUT)
        .unwrap();
    fixture
        .state
        .writer
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .flush()
        .unwrap();
    tokio::time::timeout(SUPERVISION_TEST_TIMEOUT, async {
        loop {
            if fixture.state.journal.lock().unwrap().last_sequence() > 0 {
                break;
            }
            tokio::time::sleep(SUPERVISION_TEST_POLL).await;
        }
    })
    .await
    .unwrap();
    assert!(process::is_process_alive(fixture.child_pid()));
    assert!(!fixture.state.has_termination_intent());
    fixture.state.child_waited.store(true, Ordering::Release);
}

#[test]
fn waiter_keeps_child_handle_across_panic_and_error() {
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let child = FlakyWaitChild {
        attempts: Arc::clone(&attempts),
    };
    let identity = ChildIdentity {
        pid: std::process::id(),
        start_time: process::process_start_time(std::process::id()).ok(),
    };
    let (sender, receiver) = mpsc::sync_channel(1);
    let waiter = std::thread::spawn(move || {
        let mut child = child;
        let _ = sender.send(wait_for_child(&mut child, Some(identity)));
    });
    let status = receiver
        .recv_timeout(SUPERVISION_TEST_TIMEOUT)
        .unwrap_or_else(|error| {
            panic!(
                "{WAITER_RESULT_TIMEOUT_DETAIL}: attempts={}, error={error}",
                attempts.load(Ordering::Acquire)
            )
        });
    let status = status.expect(WAITER_STATUS_MISSING_DETAIL);
    assert_eq!(status.exit_code(), FLAKY_WAIT_EXIT_CODE);
    assert_eq!(attempts.load(Ordering::Acquire), 3);
    waiter.join().unwrap();
}

#[tokio::test]
async fn missing_child_identity_finalizes_once_with_unknown_status() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    let mut events = fixture.state.events.subscribe();
    let waiter = spawn_waiter(Box::new(MissingWaitChild), Arc::clone(&fixture.state)).unwrap();
    let exit = wait_for_exit(&fixture.state).await;
    assert_eq!((exit.code, exit.signal), (None, None));
    assert!(matches!(events.recv().await, Ok(WorkerEvent::Exited(_))));
    assert!(events.try_recv().is_err());
    waiter.join().unwrap();
}

#[tokio::test]
async fn auxiliary_task_failures_leave_the_pty_and_first_reason_untouched() {
    let fixture = Fixture::new();
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(FailingIo));
    let mut tasks = JoinSet::new();
    let state = Arc::clone(&fixture.state);
    supervision::spawn_task(&mut tasks, supervision::INITIAL_INPUT_TASK, async move {
        write_initial_input(state, b"resume".to_vec()).await
    });
    let completion = tasks.join_next().await;
    handle_task_completion(
        completion,
        &mut tasks,
        &fixture.state,
        None,
        UNUSED_SHELL_PID,
    );
    supervision::spawn_task(&mut tasks, supervision::CONNECTION_TASK, async move {
        panic!("injected connection panic");
    });
    let completion = tasks.join_next().await;
    handle_task_completion(
        completion,
        &mut tasks,
        &fixture.state,
        None,
        UNUSED_SHELL_PID,
    );
    assert!(process::is_process_alive(fixture.child_pid()));
    assert!(!fixture.state.has_termination_intent());
    assert_eq!(fixture.state.termination_reason(), None);
}
