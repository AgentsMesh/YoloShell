use super::{
    attach_support::*,
    support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, create, deadline},
};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::AttachRequest;
use yolo_hub_terminal_client::Attachment;

#[tokio::test(flavor = "multi_thread")]
async fn real_public_attach_helper_fences_takeover_and_status_loss_without_stopping_shell() {
    let harness = crate::Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let state = root.path().canonicalize().unwrap().join(STATE_DIRECTORY);
    let hub = RunningHub::start(&harness, &state).await;
    let session = create(&hub, &harness).await;
    let target = session.fields().runtime.unwrap().target;
    let before = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let owner = uuid::Uuid::new_v4();
    let mut first = RunningAttach::start(&hub, target, owner, None);
    first.expect_status(StatusKind::ControlAcquired).await;
    first.input(INPUT).await;
    first.marker(MARKER).await;

    let mut observer = RunningAttach::start(&hub, target, uuid::Uuid::new_v4(), Some(OBSERVE));
    let status = observer.expect_status(StatusKind::ReadOnly).await;
    assert_eq!(status.holder_id, Some(owner.to_string()));
    observer.input(REJECTED).await;
    observer.stop(true).await;
    let mut client = hub.client().await;
    let Reply::LeaseObservation { lease: Some(lease) } = client
        .call(hub.hub, Command::ObserveLease { target }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(lease.fields().holder_id, owner.to_string());

    let replacement = uuid::Uuid::new_v4();
    let mut controller = RunningAttach::start(&hub, target, replacement, Some(TAKEOVER));
    controller
        .expect_status(StatusKind::ControlAcquired)
        .await;
    first.input(REJECTED).await;
    first.expect_status(StatusKind::ControlLost).await;
    first.stop(true).await;
    let Reply::LeaseObservation { lease: Some(lease) } = client
        .call(hub.hub, Command::ObserveLease { target }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(lease.fields().holder_id, replacement.to_string());
    controller.input(REPLACEMENT_INPUT).await;
    controller.marker(REPLACEMENT_MARKER).await;
    controller.lose_status().await;
    let Reply::LeaseObservation { lease } = client
        .call(hub.hub, Command::ObserveLease { target }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(lease.is_none());
    let after = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let before = before.runtime.unwrap();
    let after = after.runtime.unwrap();
    assert_eq!(after.runtime_epoch, before.runtime_epoch);
    assert_eq!(after.child_pid, before.child_pid);
    assert_eq!(after.worker_pid, before.worker_pid);
    assert_eq!(after.status, yolo_runtime::model::TerminalRuntimeStatus::Running);
    let output = Attachment::open(
        hub.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let snapshot = output.initial().fields().snapshot.as_ref().unwrap();
    assert!(!String::from_utf8_lossy(&snapshot.fields().data).contains(REJECTED_MARKER));
    output.detach(deadline()).await.unwrap();
    drop(client);
    hub.stop().await;
}
