use super::*;
use crate::worker::pty_child_reaper::wait_direct_child;

const WAIT_AUTHORITY_FAULT_FRAGMENT: &str = "observe PTY root exit without reaping";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn externally_reaped_root_finalizes_as_lost() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    let ownership = take_fault_ownership(&fixture);
    let root = fixture.state.pty_root_identity();
    root.signal(libc::SIGKILL).unwrap();
    assert!(wait_direct_child(root.pid).status.is_some());
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let state = Arc::downgrade(&fixture.state);
    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                state,
                Box::new(pty_exit_observer::WaitidRootExitObserver::new(root)),
                receiver,
                owner_settlement,
            )
            .run();
    });

    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;

    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert_eq!(exit.code, None);
    assert_eq!(exit.signal, None);
    assert!(!fixture.state.child_waited.load(Ordering::Acquire));
    assert!(
        settlement
            .get()
            .unwrap()
            .as_ref()
            .unwrap_err()
            .contains(WAIT_AUTHORITY_FAULT_FRAGMENT)
    );
    owner.join().unwrap();
}

#[test]
fn lost_reap_evidence_survives_owner_panic_reentry() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    let mut ownership = take_fault_ownership(&fixture);
    ownership.tree_mut().retire_session_for_test();
    let root = fixture.state.pty_root_identity();
    root.signal(libc::SIGKILL).unwrap();
    assert!(wait_direct_child(root.pid).status.is_some());
    let lost = ownership.wait_until_reaped();
    assert_eq!(lost.evidence, pty_ownership::ReapEvidence::LostUnknown);
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner = ownership.into_owner_for_test(
        Arc::downgrade(&fixture.state),
        Box::new(pty_exit_observer::WaitidRootExitObserver::new(root)),
        receiver,
        settlement,
    );

    owner.run();

    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert!(!fixture.state.child_waited.load(Ordering::Acquire));
}
