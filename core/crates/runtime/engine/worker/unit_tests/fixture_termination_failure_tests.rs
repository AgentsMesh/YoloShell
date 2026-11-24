use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

const FAILURE_AFTER_OWNER_TRANSFER: &str = "injected failure after PTY owner transfer";
const NO_WAITABLE_CHILD: libc::pid_t = -1;

#[test]
fn panic_after_owner_transfer_reaps_child_and_settles_reader_on_independent_executor() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let fixture = runtime
        .block_on(async { Fixture::with_contained_command(CommandBuilder::new(FIXTURE_COMMAND)) });
    let mut observed = None;
    let root = fixture.state.pty_root_identity();
    let panic = catch_unwind(AssertUnwindSafe(|| {
        runtime.block_on(async {
            let owned = fixture.start_termination_runtime().unwrap();
            observed = Some(Arc::clone(&owned.observation));
            panic!("{FAILURE_AFTER_OWNER_TRANSFER}");
        })
    }));
    assert!(panic.is_err());
    assert_fixture_failure_settled(&fixture, root, observed.unwrap());
}

#[tokio::test]
async fn rejected_termination_request_still_releases_all_owned_runtime_resources() {
    let fixture = Fixture::with_contained_command(CommandBuilder::new(FIXTURE_COMMAND));
    let root = fixture.state.pty_root_identity();
    let owned = fixture.start_termination_runtime().unwrap();
    let observed = Arc::clone(&owned.observation);
    let error = dispatch_test(
        &fixture.state,
        method::TERMINATE_RUNTIME_EXACT,
        &FencedTerminateRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: Uuid::new_v4(),
            expected_status: Some(TerminalRuntimeStatus::Running),
            force: false,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(root.matches());
    drop(owned);
    assert_fixture_failure_settled(&fixture, root, observed);
}

fn assert_fixture_failure_settled(
    fixture: &Fixture,
    root: process::ProcessIdentity,
    observed: FixtureTerminationObservation,
) {
    assert_eq!(observed.get(), Some(&Ok(())));
    assert!(!root.matches());
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    let progress = fixture.state.reader_lifecycle.lock_progress();
    assert!(progress.is_drained());
    assert!(progress.has_reader_supervisor_completion());
    drop(progress);
    assert!(lock_recover(&fixture.state.master).is_none());
    assert!(lock_recover(&fixture.state.writer).is_none());
    assert!(lock_recover(&fixture.state.pty_owner).is_none());
    let mut status = libc::c_int::default();
    assert_eq!(
        unsafe { libc::waitpid(root.pid.try_into().unwrap(), &mut status, libc::WNOHANG) },
        NO_WAITABLE_CHILD
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
