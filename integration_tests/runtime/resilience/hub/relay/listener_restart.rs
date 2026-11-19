use super::process::{self, RelayProcess};
use crate::hub::{
    federation_agents::launch_client,
    federation_support::{RunningNode, TEMP_ROOT, deadline, exports, listener, peer},
    local_startup_support::retain_fixture_host,
    multipath::{io, observe},
};
use std::sync::Arc;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_state::HubState;
#[path = "listener_restart_constants.rs"]
mod constants;
use constants::*;

async fn snapshot(
    process: &RelayProcess,
    peer: yolo_hub_delegation::PeerIdentity,
) -> ConfigurationSnapshot {
    let Reply::Configuration { snapshot } = process
        .client()
        .await
        .call(peer.hub(), Command::GetConfiguration {}, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    snapshot
}

#[tokio::test(flavor = "multi_thread")]
async fn source_hub_sigkill_reuses_enrolled_ephemeral_port_and_rejects_external_occupation() {
    let entry_host = crate::agent_recovery::federation_harness().await;
    let entry_hold = retain_fixture_host(&entry_host).await;
    let source_host = crate::agent_recovery::federation_harness().await;
    let source_hold = retain_fixture_host(&source_host).await;
    let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let root = directory.path().canonicalize().unwrap();
    let entry_state =
        Arc::new(HubState::open(&root.join(ENTRY_DIRECTORY), entry_host.home.path()).unwrap());
    let source_path = root.join(SOURCE_DIRECTORY);
    let source_state = HubState::open(&source_path, source_host.home.path()).unwrap();
    let entry = entry_state.peer_identity();
    let source = source_state.peer_identity();
    let source_socket = source_state.socket_path();
    drop(source_state);
    let source_configuration = Configuration {
        listen: Some(LOOPBACK.parse().unwrap()),
        peers: vec![peer(entry, None, false, vec![exports(source.hub(), false)])],
        ..Configuration::default()
    };
    let seed = process::write_configuration(&source_path, &source_configuration);
    let source_process =
        RelayProcess::start(&source_host, &source_path, &source_socket, source, &seed).await;
    let original = snapshot(&source_process, source).await;
    assert!(original.applied);
    assert_eq!(original.state.configuration, source_configuration);
    let source_address = original.listen_address.unwrap();
    let entry_listener = listener().await;
    let entry_configuration = Configuration {
        listen: Some(entry_listener.local_addr().unwrap()),
        peers: vec![peer(source, Some(source_address), true, vec![])],
        ..Configuration::default()
    };
    let entry_node = RunningNode::start(
        &entry_host,
        entry_state,
        entry_configuration,
        entry_listener,
    )
    .await;
    let agent = launch_client(
        source_process.client().await,
        source.hub(),
        &source_host,
        AGENT,
    )
    .await;
    let addresses = entry_node.wait_origins(&[source.hub()]).await;
    let original_session = source_host
        .client()
        .get_session(agent.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let mut output = io::attach(&entry_node, agent, None).await;
    let cursor = io::exchange(&entry_node, agent, &mut output).await;
    drop((entry_hold, source_hold));
    source_process.crash().await;
    io::expect_transport_loss(&mut output, agent).await;
    drop(output);
    observe::connections(&entry_node, &[]).await;
    observe::custody(&source_host, agent, &original_session).await;
    let source_process =
        RelayProcess::start_existing(&source_host, &source_path, &source_socket, source).await;
    assert_eq!(snapshot(&source_process, source).await, original);
    assert_eq!(entry_node.wait_origins(&[source.hub()]).await, addresses);
    let mut recovered = io::attach(&entry_node, agent, Some(cursor)).await;
    io::exchange(&entry_node, agent, &mut recovered).await;
    drop(recovered);
    source_process.crash().await;
    observe::connections(&entry_node, &[]).await;
    let occupied = std::net::TcpListener::bind(source_address).unwrap();
    let rejection =
        RelayProcess::expect_rejected(&source_host, &source_path, &source_socket, source).await;
    assert!(rejection.contains(OCCUPIED), "{rejection}");
    observe::custody(&source_host, agent, &original_session).await;
    drop(occupied);
    let source_process =
        RelayProcess::start_existing(&source_host, &source_path, &source_socket, source).await;
    assert_eq!(snapshot(&source_process, source).await, original);
    assert_eq!(entry_node.wait_origins(&[source.hub()]).await, addresses);
    observe::custody(&source_host, agent, &original_session).await;
    entry_node.stop().await;
    source_process.stop().await;
}
