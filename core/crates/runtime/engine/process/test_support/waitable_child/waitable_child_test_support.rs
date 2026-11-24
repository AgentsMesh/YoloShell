use std::borrow::BorrowMut;
use std::process::{Child, Command, Stdio};

const BLOCKING_TEST_PROCESS_COMMAND: &str = crate::test_executables::CAT;
const IDENTITY_CAPTURE_FAILURE: &str = "capture blocking test process identity";
const TEST_EXIT_EXECUTABLE: &str = crate::test_executables::TRUE;
const MISSING_GATE_DETAIL: &str = "test child spawn bypassed the waitable spawn gate";

pub(crate) fn spawn_waitable_test_child(mut command: impl BorrowMut<Command>) -> Child {
    let _spawn_guard = super::waitable_child_spawn_guard().unwrap();
    command.borrow_mut().spawn().unwrap()
}

pub fn spawn_waitable_test_child_in_boundary(
    mut command: impl BorrowMut<Command>,
    boundary: &super::WaitableChildSpawnBoundary<'_>,
) -> Child {
    boundary.spawn_command(command.borrow_mut()).unwrap()
}

pub(crate) fn reaped_test_process_identity() -> super::ProcessIdentity {
    let mut command = Command::new(BLOCKING_TEST_PROCESS_COMMAND);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = spawn_waitable_test_child(command);
    let identity = super::ProcessIdentity::capture(child.id()).unwrap_or_else(|error| {
        drop(child.stdin.take());
        let _ = child.wait();
        panic!("{IDENTITY_CAPTURE_FAILURE}: {error}");
    });
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
    identity
}

#[cfg(feature = "yolo-integration-tests")]
#[path = "../../integration_tests/waitable_child_test_support_tests.rs"]
mod tests;
