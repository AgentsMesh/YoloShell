use super::support::*;
use crate::Harness;
use yolo_hub_client::ClientError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;
use yolo_hub_terminal_client::Attachment;

#[tokio::test(flavor = "multi_thread")]
async fn real_hub_writer_conflict_and_service_restart_preserve_the_original_pty() {
    let harness = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let state = root.path().canonicalize().unwrap().join(STATE_DIRECTORY);
    let hub = RunningHub::start(&harness, &state).await;
    let identity = hub.hub;
    let session = create(&hub, &harness).await;
    let target = session.fields().runtime.unwrap().target;
    let mut first = hub.client().await;
    let mut second = hub.client().await;
    let mut output = Attachment::open(
        hub.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let Reply::Lease { grant: first_grant } = first
        .call(identity, acquire(target, LeaseMode::Acquire {}), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(matches!(
        second
            .call(identity, acquire(target, LeaseMode::Acquire {}), deadline())
            .await,
        Err(ClientError::Remote(PublicError::Conflict))
    ));
    let Reply::LeaseObservation {
        lease: Some(observed),
    } = second
        .call(identity, Command::ObserveLease { target }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        observed.fields().generation,
        first_grant.fields().generation
    );
    let stale = acquire(
        target,
        LeaseMode::TakeControl {
            observed_generation: observed.fields().generation + FIRST_LEASE_GENERATION,
        },
    );
    assert!(matches!(
        second.call(identity, stale, deadline()).await,
        Err(ClientError::Remote(PublicError::Conflict))
    ));
    let takeover = acquire(
        target,
        LeaseMode::TakeControl {
            observed_generation: observed.fields().generation,
        },
    );
    let Reply::Lease { grant } = second.call(identity, takeover, deadline()).await.unwrap() else {
        panic!()
    };
    assert!(
        first
            .call(identity, input(first_grant, REJECTED_COMMAND), deadline())
            .await
            .is_err()
    );
    second
        .call(identity, input(grant, OUTPUT_COMMAND), deadline())
        .await
        .unwrap();
    let mut bytes = Vec::new();
    while !String::from_utf8_lossy(&bytes).contains(OUTPUT_MARKER) {
        match output.next_event(deadline()).await.unwrap() {
            TerminalEvent::Output { chunk } => bytes.extend_from_slice(&chunk.fields().data),
            TerminalEvent::Heartbeat { .. } => {}
            event => panic!("unexpected event: {event:?}"),
        }
        assert!(bytes.len() < MAX_OBSERVED_BYTES);
    }
    let cursor = output.cursor();
    drop(first);
    drop(second);
    drop(output);
    hub.stop().await;
    let still_running = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert_eq!(
        still_running.runtime.as_ref().unwrap().runtime_epoch,
        target.runtime_epoch.as_uuid()
    );
    let restored = RunningHub::start(&harness, &state).await;
    assert_eq!(restored.hub, identity);
    let mut output = Attachment::open(
        restored.client().await,
        AttachRequest::new(target, Some(cursor)).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let mut client = restored.client().await;
    client
        .call(identity, input(grant, EXIT_COMMAND), deadline())
        .await
        .unwrap();
    loop {
        match output.next_event(deadline()).await.unwrap() {
            TerminalEvent::Output { .. } | TerminalEvent::Heartbeat { .. } => {}
            TerminalEvent::Exited { cursor, .. } => {
                assert_eq!(cursor.target, target);
                break;
            }
            event => panic!("exit cannot be inferred from {event:?}"),
        }
    }
    drop(output);
    drop(client);
    restored.stop().await;
}
