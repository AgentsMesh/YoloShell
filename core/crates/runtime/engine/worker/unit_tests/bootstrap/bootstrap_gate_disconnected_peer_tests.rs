use super::tests::{
    TEST_MARKER, prepare_gate_for_test, test_spec, test_wrapper_command, wait_for_child,
};
use super::*;
use crate::process::ProcessIdentity;

const TEST_SOCKET_FILE: &str = "disconnected-peer.sock";
const DIRECT_TEST_SOCKET_FILE: &str = "direct-disconnected-peer.sock";
const TEST_MARKER_FILE: &str = "disconnected-peer.marker";

#[tokio::test]
async fn disconnected_current_process_peer_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(DIRECT_TEST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let connected = UnixStream::connect(&socket).await.unwrap();
    let (accepted, _address) = listener.accept().await.unwrap();
    drop(connected);
    let accepted = LocalUnixStream::from(accepted);

    assert!(!accepted_peer_matches(&accepted, std::process::id()).unwrap());
}

#[tokio::test]
async fn disconnected_candidate_does_not_terminate_gate_acceptance() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(TEST_SOCKET_FILE);
    let marker = directory.path().join(TEST_MARKER_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let disconnected = UnixStream::connect(&socket).await.unwrap();
    drop(disconnected);

    let spec = test_spec(socket, &marker);
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
    assert_eq!(std::fs::read(marker).unwrap(), TEST_MARKER);
}
