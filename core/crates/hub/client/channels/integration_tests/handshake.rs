use crate::{constants::*, harness::Harness, peer::*};
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::Command;

#[tokio::test]
async fn queued_commands_are_rejected_during_handshake_and_shutdown_cancels_it() {
    let mut harness = Harness::start();
    let (stream, _) = tokio::time::timeout_at(deadline(), harness.listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    let request = receive(&mut peer).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    harness.send(None, Command::Hello {}).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::NotConnected),
            connected: false,
            ..
        }
    ));
    harness.finish().await;
    eof(&mut peer).await;
}

#[tokio::test]
async fn failed_connection_is_reported_without_a_ready_generation() {
    use tokio::sync::{mpsc, watch};
    use yolo_hub_client_channels::{Endpoint, run};
    let root = tempfile::tempdir_in(ROOT).unwrap();
    let endpoint = Endpoint::local(root.path().join(SOCKET), yolo_hub_identity::HubId::new());
    let (commands, requests) = mpsc::channel(CAPACITY);
    let (maintenance, renewals) = mpsc::channel(CAPACITY);
    let (refresh, refreshes) = watch::channel(None);
    let (updates, mut controls) = mpsc::channel(CAPACITY);
    let (catalogs, _catalogs) = mpsc::channel(CAPACITY);
    let mut owner = Box::pin(run(
        endpoint, requests, renewals, refreshes, updates, catalogs,
    ));
    tokio::select! {
        _ = &mut owner => panic!(),
        event = controls.recv() => assert!(matches!(event, Some(ControlEvent::Unreachable { .. }))),
    }
    drop(commands);
    drop(maintenance);
    drop(refresh);
    tokio::time::timeout(SHUTDOWN_TIMEOUT, owner).await.unwrap();
}
