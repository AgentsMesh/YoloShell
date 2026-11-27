use super::tests::{
    TEST_PROCESS_TIMEOUT, prepare_gate_for_test, test_spec, test_wrapper_command, wait_for_child,
};
use super::*;
use crate::process::ProcessIdentity;
use std::time::Instant as StdInstant;

const TEST_IMMEDIATE_COMMAND: &str = crate::test_executables::TRUE;
const TEST_MISSING_COMMAND: &str = "/definitely/missing/yolo-bootstrap-command";
const TEST_MARKER_TIMEOUT_DETAIL: &str = "pre-exec marker was not written";
const TEST_PROCESS_CAPABILITY_TIMEOUT_DETAIL: &str =
    "bootstrap process capability did not release during kernel exit";

#[tokio::test]
async fn successful_exec_is_confirmed_even_when_command_exits_immediately() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("immediate.sock");
    let marker = directory.path().join("immediate.marker");
    let listener = UnixListener::bind(&socket).unwrap();
    let mut spec = test_spec(socket, &marker);
    spec.command = TEST_IMMEDIATE_COMMAND.into();
    spec.args.clear();
    let mut child = crate::process::spawn_waitable_test_child(&mut test_wrapper_command(&spec));
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let gate = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .unwrap();
    let gate = prepare_gate_for_test(gate, identity).await;

    let confirmation = gate
        .release()
        .await
        .unwrap()
        .confirm_exec(crate::process::BootstrapExecTransitionAuthority::for_test())
        .await
        .unwrap();

    assert!(matches!(confirmation, BootstrapExecConfirmation::Confirmed));
    assert!(wait_for_child(&mut child).success());
    assert!(!marker.exists());
}

#[tokio::test]
async fn child_killed_after_go_before_exec_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("killed.sock");
    let marker = directory.path().join("killed.marker");
    let pause_marker = directory.path().join("pre-exec.marker");
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket, &marker);
    let mut command = test_wrapper_command(&spec);
    command.env(BOOTSTRAP_TEST_PRE_EXEC_MARKER_ENV, &pause_marker);
    let mut child = crate::process::spawn_waitable_test_child(command);
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let gate = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .unwrap();
    let gate = prepare_gate_for_test(gate, identity).await;
    let released = gate.release().await.unwrap();
    wait_for_marker(&pause_marker);
    child.kill().unwrap();

    let error = released
        .confirm_exec(crate::process::BootstrapExecTransitionAuthority::for_test())
        .await
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT)
    );
    assert!(!wait_for_child(&mut child).success());
    assert!(!marker.exists());
}

#[tokio::test]
async fn exec_failure_is_reported_after_go() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("exec-failure.sock");
    let marker = directory.path().join("exec-failure.marker");
    let listener = UnixListener::bind(&socket).unwrap();
    let mut spec = test_spec(socket, &marker);
    spec.command = TEST_MISSING_COMMAND.into();
    let mut child = crate::process::spawn_waitable_test_child(&mut test_wrapper_command(&spec));
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let gate = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .unwrap();
    #[cfg(target_os = "macos")]
    let process_capability = gate.descriptor_capability();
    let gate = prepare_gate_for_test(gate, identity).await;

    let confirmation = gate
        .release()
        .await
        .unwrap()
        .confirm_exec(crate::process::BootstrapExecTransitionAuthority::for_test())
        .await
        .unwrap();

    let BootstrapExecConfirmation::ExplicitFailure(failure) = confirmation else {
        panic!("{BOOTSTRAP_EXEC_FAILURE_CONTEXT}");
    };
    assert!(!failure.detail().is_empty());
    assert!(!wait_for_child(&mut child).success());
    #[cfg(target_os = "macos")]
    wait_for_process_capability_release(&process_capability);
    assert!(!marker.exists());
}

#[cfg(target_os = "macos")]
fn wait_for_process_capability_release(
    capability: &crate::process::MacosProcessCapabilityManagerReader,
) {
    let deadline = StdInstant::now() + TEST_PROCESS_TIMEOUT;
    loop {
        let observation = capability.liveness_observation();
        if observation
            == Some(crate::process::MacosProcessCapabilityObservation::ReleasedAfterRootExit)
        {
            return;
        }
        assert!(
            StdInstant::now() < deadline,
            "{TEST_PROCESS_CAPABILITY_TIMEOUT_DETAIL}: {observation:?}"
        );
        std::thread::yield_now();
    }
}

fn wait_for_marker(marker: &std::path::Path) {
    let deadline = StdInstant::now() + TEST_PROCESS_TIMEOUT;
    loop {
        match std::fs::read(marker) {
            Ok(content) if content == BOOTSTRAP_TEST_PRE_EXEC_MARKER_CONTENT => return,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("{TEST_MARKER_TIMEOUT_DETAIL}: {error}"),
        }
        assert!(StdInstant::now() < deadline, "{TEST_MARKER_TIMEOUT_DETAIL}");
        std::thread::yield_now();
    }
}
