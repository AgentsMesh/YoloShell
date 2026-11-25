use super::*;
use crate::process::ProcessIdentity;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;
use uuid::Uuid;
use yolo_coverage_test_support::discard_reaped_kill_profile;

pub(super) const TEST_COMMAND: &str = "/bin/sh";
const TEST_COMMAND_FLAG: &str = "-c";
const TEST_SCRIPT_NAME: &str = "yolo-bootstrap-gate-test";
const TEST_SCRIPT: &str = "printf x > \"$1\"";
pub(super) const TEST_MARKER: &[u8] = b"x";
const TEST_WRONG_PEERS: usize = 8;
pub(super) const TEST_PROCESS_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) fn test_spec(socket_path: PathBuf, marker: &std::path::Path) -> BootstrapChildSpec {
    BootstrapChildSpec {
        socket_path,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        token: Uuid::new_v4(),
        command: TEST_COMMAND.into(),
        args: vec![
            TEST_COMMAND_FLAG.into(),
            TEST_SCRIPT.into(),
            TEST_SCRIPT_NAME.into(),
            marker.to_string_lossy().into_owned(),
        ],
    }
}

pub(super) fn test_wrapper_command(spec: &BootstrapChildSpec) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            BOOTSTRAP_TEST_EXACT_ARGUMENT,
            BOOTSTRAP_TEST_ENTRY,
            BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
        ])
        .env(
            BOOTSTRAP_GATE_SPEC_ENV,
            super::super::bootstrap_spec_codec::encode(spec).unwrap(),
        );
    command
}

fn spawn_test_wrapper(spec: &BootstrapChildSpec) -> Child {
    crate::process::spawn_waitable_test_child(&mut test_wrapper_command(spec))
}

pub(super) fn wait_for_child(child: &mut Child) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + TEST_PROCESS_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            discard_reaped_kill_profile(child.id(), status).unwrap();
            return status;
        }
        assert!(std::time::Instant::now() < deadline, "wrapper did not exit");
        std::thread::yield_now();
    }
}

pub(super) async fn prepare_gate_for_test(
    gate: ValidatedBootstrapGate,
    identity: ProcessIdentity,
) -> PreparedBootstrapGate {
    #[cfg(target_os = "macos")]
    {
        gate.descriptor_capability()
            .arm_liveness_for_test(identity)
            .unwrap();
        assert_eq!(
            gate.descriptor_capability().liveness_observation(),
            Some(crate::process::MacosProcessCapabilityObservation::WriterPresent)
        );
    }
    #[cfg(not(target_os = "macos"))]
    let _ = identity;
    gate.prepare().await.unwrap()
}
#[tokio::test]
async fn command_cannot_execute_before_pending_intent_and_go() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("gate.sock");
    let marker = directory.path().join("command.marker");
    let intent = directory.path().join("pending.intent");
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket, &marker);
    let mut child = spawn_test_wrapper(&spec);
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let gate = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .unwrap();
    let gate = prepare_gate_for_test(gate, identity).await;
    assert!(!marker.exists());
    let mut file = std::fs::File::create(&intent).unwrap();
    file.write_all(b"pending").unwrap();
    file.sync_all().unwrap();
    assert!(!marker.exists());
    let confirmation = gate
        .release()
        .await
        .unwrap()
        .confirm_exec(crate::process::BootstrapExecTransitionAuthority::for_test())
        .await
        .unwrap();
    assert!(matches!(confirmation, BootstrapExecConfirmation::Confirmed));
    assert!(wait_for_child(&mut child).success());
    assert_eq!(std::fs::read(marker).unwrap(), TEST_MARKER);
}

#[tokio::test]
async fn unrelated_backlog_peers_are_discarded_before_child_hello() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("backlog.sock");
    let marker = directory.path().join("backlog.marker");
    let listener = UnixListener::bind(&socket).unwrap();
    let mut wrong = Vec::new();
    for _ in 0..TEST_WRONG_PEERS {
        wrong.push(UnixStream::connect(&socket).await.unwrap());
    }
    let spec = test_spec(socket, &marker);
    let mut child = spawn_test_wrapper(&spec);
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let gate = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .unwrap();
    let gate = prepare_gate_for_test(gate, identity).await;
    assert!(!marker.exists());
    drop(wrong);
    let confirmation = gate
        .release()
        .await
        .unwrap()
        .confirm_exec(crate::process::BootstrapExecTransitionAuthority::for_test())
        .await
        .unwrap();
    assert!(matches!(confirmation, BootstrapExecConfirmation::Confirmed));
    assert!(wait_for_child(&mut child).success());
    assert_eq!(std::fs::read(marker).unwrap(), TEST_MARKER);
}

#[tokio::test]
async fn matching_pid_with_wrong_token_is_rejected_without_go() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("wrong-token.sock");
    let marker = directory.path().join("wrong-token.marker");
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket, &marker);
    let mut expected = test_spec(spec.socket_path.clone(), &marker);
    expected.terminal_id = spec.terminal_id;
    expected.runtime_epoch = spec.runtime_epoch;
    expected.worker_id = spec.worker_id;
    let mut child = spawn_test_wrapper(&spec);
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let error = accept_bootstrap_gate(&listener, &expected.expected(identity))
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains(BOOTSTRAP_GATE_HELLO_CONTEXT));
    assert!(!marker.exists());
    child.kill().unwrap();
    let _ = wait_for_child(&mut child);
}

#[tokio::test]
async fn eof_before_go_fails_closed() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("eof.sock");
    let marker = directory.path().join("eof.marker");
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket, &marker);
    let mut child = spawn_test_wrapper(&spec);
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let gate = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .unwrap();
    drop(gate);
    assert!(!wait_for_child(&mut child).success());
    assert!(!marker.exists());
}
