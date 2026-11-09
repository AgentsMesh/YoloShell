use crate::worker::tests::*;
const INJECTED_OWNER_FAULT: &str = "injected PTY ownership observation fault";
const INJECTED_OWNER_PANIC: &str = "injected PTY owner panic";
pub(super) const OWNER_FAULT_TIMEOUT: Duration = WORKER_TERMINATION_TIMEOUT;

pub(super) struct FaultingRootObserver;
struct PanickingRootObserver;

impl pty_exit_observer::RootExitObserver for FaultingRootObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &process::RootWaitAuthority,
    ) -> Result<Option<process::UnreapedRootExit>> {
        anyhow::bail!(INJECTED_OWNER_FAULT)
    }
}

impl pty_exit_observer::RootExitObserver for PanickingRootObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &process::RootWaitAuthority,
    ) -> Result<Option<process::UnreapedRootExit>> {
        panic!("{INJECTED_OWNER_PANIC}")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn natural_owner_fault_is_durable_as_a_lost_exit() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .owner_fault_before_cleanup
        .arm();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let ownership = take_fault_ownership(&fixture);
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let state = Arc::downgrade(&fixture.state);
    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(state, Box::new(FaultingRootObserver), receiver, settlement)
            .run();
    });

    let hook_state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || {
        hook_state
            .control_test_hooks
            .owner_fault_before_cleanup
            .wait_until_reached();
    })
    .await
    .unwrap();

    let provisional: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(provisional.reason, ExitReason::Lost);
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Finalizing);
    assert!(!fixture.state.child_waited.load(Ordering::Acquire));
    fixture
        .state
        .control_test_hooks
        .owner_fault_before_cleanup
        .resume();
    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;
    assert_eq!(
        lock_recover(&fixture.state.exit).as_ref().unwrap().reason,
        ExitReason::Lost
    );
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    owner.join().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_force_survives_owner_panic_before_accept() {
    let fixture = Fixture::new();
    let controller_permit = fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&controller_permit, ExitReason::UserTerminate);
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let ownership = take_fault_ownership(&fixture);
    let (commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let (response, completion) = tokio::sync::oneshot::channel();
    commands
        .send(pty_owner_handle::PtyOwnerCommand::Terminate {
            force: true,
            response,
        })
        .unwrap();
    let state = Arc::downgrade(&fixture.state);
    let owner_settlement = Arc::clone(&settlement);
    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                state,
                Box::new(PanickingRootObserver),
                receiver,
                owner_settlement,
            )
            .run();
    });

    let completion = tokio::time::timeout(OWNER_FAULT_TIMEOUT, completion)
        .await
        .unwrap()
        .unwrap();
    assert!(completion.unwrap_err().contains(INJECTED_OWNER_PANIC));
    assert_eq!(
        fixture.state.termination_reason(),
        Some(ExitReason::UserTerminate)
    );
    drop(controller_permit);
    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;

    let exit = lock_recover(&fixture.state.exit);
    let exit = exit.as_ref().unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert_eq!(exit.signal, Some(libc::SIGKILL));
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert!(settlement.get().unwrap().is_err());
    owner.join().unwrap();
}

include!("pty_owner_persistence_fault.rs");
include!("../output/pty_output_progress_owner_tests.rs");
include!("pty_owner_staging_tests.rs");
include!("pty_owner_root_exit_branch_tests.rs");
include!("pty_owner_deferred_state_tests.rs");

#[path = "pty_owner_wait_authority_loss.rs"]
mod wait_authority_loss_tests;
