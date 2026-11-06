use super::super::{
    local_startup_support::*,
    support::{MAX_OBSERVED_BYTES, acquire, input},
};
use crate::Harness;
use yolo_hub_client::Client;
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, LeaseMode, TerminalEvent};
use yolo_hub_terminal_client::Attachment;

const SESSION_NAME: &str = "bootstrap survival";
const INPUT: &[u8] = b"YOLO_BOOTSTRAP_SURVIVAL\n";
const MARKER: &str = "YOLO_BOOTSTRAP_SURVIVAL";

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_restarts_only_the_hub_and_preserves_a_live_session_and_writer() {
    let harness = Harness::start().await;
    let before = harness.create_cat(SESSION_NAME).await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let state = root
        .path()
        .canonicalize()
        .unwrap()
        .join(STATE_DIRECTORY)
        .join(LONG_STATE_COMPONENT.repeat(LONG_STATE_REPETITIONS));
    let first = bootstrap(harness.home.path(), &state).await;
    assert!(!first.socket_path.starts_with(&state));
    assert!(!state.join(yolo_hub_state::HUB_SOCKET_FILE).exists());
    let mut services = Services::capture(harness.home.path(), &first).await;
    super::connector::assert_empty(&first).await;
    let address = TerminalAddress {
        hub_id: first.entry_hub,
        terminal_id: TerminalId::from_uuid(before.manifest.terminal_id).unwrap(),
    };
    let mut client = Client::connect_local(&first.socket_path, first.entry_hub, deadline())
        .await
        .unwrap();
    let Reply::Terminal { terminal: session } = client
        .call(first.entry_hub, Command::Get { address }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let target = session.fields().runtime.unwrap().target;
    assert_eq!(
        target.runtime_epoch.as_uuid(),
        before.runtime.as_ref().unwrap().runtime_epoch
    );
    let Reply::Lease { grant } = client
        .call(
            first.entry_hub,
            acquire(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    drop(client);
    services.hub.signal(libc::SIGTERM).unwrap();
    crate::wait_for_identity_gone(services.hub).await;
    let second = bootstrap(harness.home.path(), &state).await;
    let (host, hub) = Services::inspect(harness.home.path(), &second).await;
    assert_eq!(host, services.host);
    assert_ne!(hub, services.hub);
    services.hub = hub;
    assert_eq!(first, second);
    super::connector::assert_empty(&second).await;
    let after = harness
        .client()
        .get_session(before.manifest.terminal_id)
        .await
        .unwrap();
    assert_eq!(
        after.runtime.as_ref().unwrap().runtime_epoch,
        target.runtime_epoch.as_uuid()
    );
    assert_eq!(
        after.runtime.as_ref().unwrap().worker_pid,
        before.runtime.as_ref().unwrap().worker_pid
    );
    let mut client = Client::connect_local(&second.socket_path, second.entry_hub, deadline())
        .await
        .unwrap();
    let mut output = Attachment::open(
        Client::connect_local(&second.socket_path, second.entry_hub, deadline())
            .await
            .unwrap(),
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    client
        .call(second.entry_hub, input(grant, INPUT), deadline())
        .await
        .unwrap();
    let mut observed = Vec::new();
    tokio::time::timeout_at(deadline(), async {
        while !String::from_utf8_lossy(&observed).contains(MARKER) {
            if let TerminalEvent::Output { chunk } = output.next_event(deadline()).await.unwrap() {
                observed.extend_from_slice(&chunk.fields().data);
                assert!(observed.len() <= MAX_OBSERVED_BYTES);
            }
        }
    })
    .await
    .unwrap();
    drop(output);
    drop(client);
    services.stop().await;
}
