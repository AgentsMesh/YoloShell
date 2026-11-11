use super::{constants::*, fixture::Fixture, inspection};
use crate::hub::support::{MAX_OBSERVED_BYTES, acquire, input};
use yolo_hub_client::Client;
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, LeaseMode, TerminalEvent};
use yolo_hub_terminal_client::Attachment;

#[tokio::test(flavor = "multi_thread")]
async fn remote_bootstrap_caller_exit_preserves_existing_worker_shell_and_terminal_control() {
    let harness = crate::Harness::start().await;
    let before = harness.create_cat(SESSION_NAME).await;
    let original = before.runtime.as_ref().unwrap();
    let worker = crate::ProcessIdentity::capture(original.worker_pid).unwrap();
    let shell = crate::ProcessIdentity::capture(original.child_pid).unwrap();
    let mut fixture = Fixture::new(Some(harness.home.path()));
    let bundle = fixture.install(false).await;
    let request = fixture.request(bundle.clone(), None);
    let receipt = fixture.bootstrap(&request).await;
    assert_eq!(fixture.identities().0, harness.host_identity());
    inspection::build(&fixture, &receipt).await;
    let replay = fixture.request(bundle, Some(receipt.fields().peer));
    let second = fixture.bootstrap(&replay).await;
    inspection::same_processes(&fixture, &second).await;
    let after = harness
        .client()
        .get_session(before.manifest.terminal_id)
        .await
        .unwrap();
    let current = after.runtime.as_ref().unwrap();
    assert_eq!(current.runtime_epoch, original.runtime_epoch);
    assert_eq!(current.worker_pid, original.worker_pid);
    assert_eq!(current.child_pid, original.child_pid);
    assert!(worker.matches() && shell.matches());
    let connection = fixture.connection(receipt.fields().peer);
    let mut client =
        Client::connect_local(&connection.socket_path, connection.entry_hub, deadline())
            .await
            .unwrap();
    let address = TerminalAddress {
        hub_id: connection.entry_hub,
        terminal_id: TerminalId::from_uuid(before.manifest.terminal_id).unwrap(),
    };
    let Reply::Terminal { terminal: session } = client
        .call(connection.entry_hub, Command::Get { address }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let target = session.fields().runtime.unwrap().target;
    let Reply::Lease { grant } = client
        .call(
            connection.entry_hub,
            acquire(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let mut output = Attachment::open(
        Client::connect_local(&connection.socket_path, connection.entry_hub, deadline())
            .await
            .unwrap(),
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    client
        .call(
            connection.entry_hub,
            input(grant, OUTPUT_COMMAND),
            deadline(),
        )
        .await
        .unwrap();
    let mut observed = Vec::new();
    tokio::time::timeout_at(deadline(), async {
        while !String::from_utf8_lossy(&observed).contains(OUTPUT_MARKER) {
            if let TerminalEvent::Output { chunk } = output.next_event(deadline()).await.unwrap() {
                observed.extend_from_slice(&chunk.fields().data);
                assert!(observed.len() <= MAX_OBSERVED_BYTES);
            }
        }
    })
    .await
    .unwrap();
    client
        .call(connection.entry_hub, Command::Release { grant }, deadline())
        .await
        .unwrap();
    drop(output);
    drop(client);
    fixture.finish().await;
    assert!(harness.host_identity().matches() && worker.matches() && shell.matches());
}
