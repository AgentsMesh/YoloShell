use super::tests::{test_spec, test_wrapper_command, wait_for_child};
use super::*;
use crate::process::ProcessIdentity;

const SWAPPED_ACK_REQUEST: &str = "requested";
const WRONG_ACK_SOCKET_FILE: &str = "wrong-ack.sock";
const WRONG_ACK_MARKER_FILE: &str = "wrong-ack.marker";
const HELLO_FIRST_SOCKET_FILE: &str = "hello-first.sock";
const HELLO_FIRST_MARKER_FILE: &str = "hello-first.marker";
const UNARMED_SOCKET_FILE: &str = "unarmed-capability.sock";
const UNARMED_MARKER_FILE: &str = "unarmed-capability.marker";
#[cfg(target_os = "linux")]
const PROCESS_TASKS_ROOT: &str = "/proc";
#[cfg(target_os = "linux")]
const PROCESS_TASKS_DIRECTORY: &str = "task";

#[cfg(target_os = "macos")]
#[tokio::test]
async fn swapped_capability_ack_is_rejected_without_go() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(WRONG_ACK_SOCKET_FILE);
    let marker = directory.path().join(WRONG_ACK_MARKER_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket, &marker);
    let mut command = test_wrapper_command(&spec);
    command.env(BOOTSTRAP_TEST_BAD_ACK_ENV, SWAPPED_ACK_REQUEST);
    let mut child = crate::process::spawn_waitable_test_child(command);
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let error = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_GATE_CAPABILITY_CONTEXT)
    );
    assert!(!wait_for_child(&mut child).success());
    assert!(!marker.exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn preparation_is_rejected_until_process_capability_watcher_is_armed() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(UNARMED_SOCKET_FILE);
    let marker = directory.path().join(UNARMED_MARKER_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket, &marker);
    let mut child = crate::process::spawn_waitable_test_child(&mut test_wrapper_command(&spec));
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let gate = accept_bootstrap_gate(&listener, &spec.expected(identity))
        .await
        .unwrap();
    assert_eq!(gate.descriptor_capability().liveness_observation(), None);

    let error = gate.prepare().await.err().unwrap();

    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_GATE_CAPABILITY_UNPREPARED_DETAIL)
    );
    assert!(!wait_for_child(&mut child).success());
    assert!(!marker.exists());
}

#[tokio::test]
async fn child_sends_authenticated_hello_before_waiting_for_capability() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(HELLO_FIRST_SOCKET_FILE);
    let marker = directory.path().join(HELLO_FIRST_MARKER_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket, &marker);
    let mut child = crate::process::spawn_waitable_test_child(&mut test_wrapper_command(&spec));
    let (mut stream, _) = listener.accept().await.unwrap();
    let hello: BootstrapHello = read_frame(&mut stream).await.unwrap();
    assert_eq!(hello.authentication(), spec.authentication());
    #[cfg(target_os = "macos")]
    assert_eq!(hello.exec_thread_id, child.id());
    #[cfg(target_os = "linux")]
    {
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let thread = std::path::Path::new(PROCESS_TASKS_ROOT)
            .join(child.id().to_string())
            .join(PROCESS_TASKS_DIRECTORY)
            .join(hello.exec_thread_id.to_string());
        assert!(thread.is_dir());
        assert!(identity.matches());
    }
    drop(stream);
    assert!(!wait_for_child(&mut child).success());
    assert!(!marker.exists());
}
