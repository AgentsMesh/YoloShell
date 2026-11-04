use crate::worker::tests::*;
const STALLED_INPUT_COMMAND: &str = "/usr/bin/tail";
const STALLED_INPUT_FOLLOW_ARGUMENT: &str = "-f";
const STALLED_INPUT_SOURCE: &str = "/dev/null";
const SATURATING_INPUT_DIVISOR: usize = 2;
const SATURATING_INPUT_BYTES: usize = crate::protocol::MAX_FRAME_BYTES / SATURATING_INPUT_DIVISOR;
const PTY_MUTATION_PROGRESS_TIMEOUT: Duration = WORKER_OPERATION_TIMEOUT;
const PTY_TERMINATION_PROGRESS_TIMEOUT: Duration = WORKER_TERMINATION_TIMEOUT;
const TERMIOS_SUCCESS: i32 = 0;

fn stalled_input_fixture() -> Fixture {
    let mut command = CommandBuilder::new(STALLED_INPUT_COMMAND);
    command.arg(STALLED_INPUT_FOLLOW_ARGUMENT);
    command.arg(STALLED_INPUT_SOURCE);
    let fixture = Fixture::with_command(command);
    configure_raw_pty(&fixture.state);
    fixture
}

fn configure_raw_pty(state: &WorkerState) {
    let master = lock_recover(&state.master);
    let fd = master.as_ref().unwrap().as_raw_fd().unwrap();
    let mut attributes = std::mem::MaybeUninit::<libc::termios>::uninit();
    assert_eq!(
        unsafe { libc::tcgetattr(fd, attributes.as_mut_ptr()) },
        TERMIOS_SUCCESS
    );
    let mut attributes = unsafe { attributes.assume_init() };
    unsafe { libc::cfmakeraw(&mut attributes) };
    assert_eq!(
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &attributes) },
        TERMIOS_SUCCESS
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saturated_pty_input_cannot_starve_takeover_or_termination() {
    let fixture = stalled_input_fixture();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request("saturated"), false)
        .await
        .unwrap();
    let input_state = Arc::clone(&fixture.state);
    let request = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: vec![b'x'; SATURATING_INPUT_BYTES],
    };
    fixture.state.control_test_hooks.input_before_delivery.arm();
    let input = tokio::spawn(async move { input_state.write_input(controller, &request).await });
    fixture
        .state
        .control_test_hooks
        .input_before_delivery
        .wait_until_reached()
        .await;
    let takeover_state = Arc::clone(&fixture.state);
    let mut takeover_request = fixture.lease_request("replacement");
    takeover_request.generation = Some(lease.generation);
    let takeover = tokio::spawn(async move {
        tokio::time::timeout(
            PTY_MUTATION_PROGRESS_TIMEOUT,
            takeover_state.acquire_lease(controller, &takeover_request, true),
        )
        .await
    });
    let system_state = Arc::clone(&fixture.state);
    let system_waiter = tokio::spawn(async move {
        tokio::time::timeout(PTY_MUTATION_PROGRESS_TIMEOUT, async {
            drop(system_state.admit_system_mutation().await);
        })
        .await
    });
    fixture
        .state
        .control_test_hooks
        .input_before_delivery
        .resume();
    let error = tokio::time::timeout(PTY_MUTATION_PROGRESS_TIMEOUT, input)
        .await
        .expect("saturated PTY input blocked the worker mutation gate")
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    let replacement = tokio::time::timeout(PTY_MUTATION_PROGRESS_TIMEOUT, takeover)
        .await
        .expect("writer takeover starved behind saturated PTY input")
        .unwrap()
        .expect("writer takeover exceeded its internal deadline")
        .unwrap();
    assert!(replacement.generation > lease.generation);
    tokio::time::timeout(PTY_MUTATION_PROGRESS_TIMEOUT, system_waiter)
        .await
        .expect("system mutation starved behind saturated PTY input")
        .unwrap()
        .expect("system mutation exceeded its internal deadline");
    fixture.state.reader_lifecycle.mark_drained();
    fixture.start_child_waiter();
    tokio::time::timeout(PTY_TERMINATION_PROGRESS_TIMEOUT, async {
        let permit = fixture.state.admit_system_mutation().await;
        terminate_pty_processes(&fixture.state, permit, true).await
    })
    .await
    .expect("PTY termination starved after saturated input")
    .unwrap();
    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::Exited,
        PTY_TERMINATION_PROGRESS_TIMEOUT,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saturated_pty_writer_finalization_is_bounded() {
    let fixture = stalled_input_fixture();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request("finalizer"), false)
        .await
        .unwrap();
    let request = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: vec![b'x'; SATURATING_INPUT_BYTES],
    };
    assert!(
        fixture
            .state
            .write_input(controller, &request)
            .await
            .is_err()
    );
    fixture.state.reader_lifecycle.mark_drained();
    let state = Arc::clone(&fixture.state);
    let finalizer = std::thread::spawn(move || {
        finalize_child_exit(&state, None, None, Duration::ZERO);
    });
    tokio::time::timeout(
        PTY_TERMINATION_PROGRESS_TIMEOUT,
        tokio::task::spawn_blocking(move || finalizer.join().unwrap()),
    )
    .await
    .expect("saturated PTY writer blocked finalization")
    .unwrap();
    assert!(lock_recover(&fixture.state.writer).is_none());
    assert!(lock_recover(&fixture.state.master).is_none());
    let permit = tokio::time::timeout(
        PTY_MUTATION_PROGRESS_TIMEOUT,
        fixture.state.admit_system_mutation(),
    )
    .await
    .expect("finalization did not release the worker mutation gate");
    drop(permit);
}
