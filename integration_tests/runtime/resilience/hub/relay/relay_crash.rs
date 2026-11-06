use super::{
    federation_agents::{launch, launch_client},
    federation_support::*,
    local_startup_support::retain_fixture_host,
    multipath::{io, observe},
};
use std::sync::Arc;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

#[path = "relay_crash_constants.rs"]
mod constants;
#[path = "listener_restart.rs"]
mod listener_restart;
#[path = "relay_crash_observe.rs"]
mod observations;
#[path = "relay_crash_process.rs"]
pub(super) mod process;
use constants::*;
use process::RelayProcess;

#[ignore = "real Hub process needs an SSH profile to dial C after ADR 0036; ssh_profile_live covers the managed broker path"]
#[tokio::test(flavor = "multi_thread")]
async fn independent_relay_sigkill_preserves_local_and_origin_agents_and_reconnects() {
    let a_host = crate::agent_recovery::federation_harness().await;
    let a_hold = retain_fixture_host(&a_host).await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let b_hold = retain_fixture_host(&b_host).await;
    let c_host = crate::agent_recovery::federation_harness().await;
    let c_hold = retain_fixture_host(&c_host).await;
    let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let root = directory.path().canonicalize().unwrap();
    let a_state =
        Arc::new(HubState::open(&root.join(ENTRY_DIRECTORY), a_host.home.path()).unwrap());
    let b_path = root.join(RELAY_DIRECTORY);
    let b_state = HubState::open(&b_path, b_host.home.path()).unwrap();
    let c_state =
        Arc::new(HubState::open(&root.join(ORIGIN_DIRECTORY), c_host.home.path()).unwrap());
    let (a, b, c) = (
        a_state.peer_identity(),
        b_state.peer_identity(),
        c_state.peer_identity(),
    );
    let b_socket = b_state.socket_path();
    let b_peer_socket = b_state.peer_socket().unwrap();
    drop(b_state);
    let a_config = Configuration {
        trusted_origins: vec![c],
        file_roots: vec![],
        peers: vec![peer_at_socket(b, b_peer_socket.clone(), true, vec![])],
    };
    let b_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![
            peer(
                a,
                None,
                false,
                vec![exports(b.hub(), false), exports(c.hub(), false)],
            ),
            peer(c, Some(&c_state), true, vec![]),
        ],
    };
    let c_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer(b, None, false, vec![exports(c.hub(), true)])],
    };
    let config_path = process::write_configuration(&b_path, &b_config);
    let a_node = RunningNode::start(&a_host, a_state, a_config).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config).await;
    let b_node = RelayProcess::start(&b_host, &b_path, &b_socket, b, &config_path).await;
    let local = launch_client(b_node.client().await, b.hub(), &b_host, RELAY_AGENT).await;
    let remote = launch(&c_node, &c_host, ORIGIN_AGENT).await;
    let addresses = a_node.wait_origins(&[b.hub(), c.hub()]).await;
    let snapshots = a_node.agents().await;
    let local_agent = snapshots
        .iter()
        .find(|agent| agent.fields().terminal == local)
        .unwrap()
        .cursor()
        .target();
    let remote_agent = snapshots
        .iter()
        .find(|agent| agent.fields().terminal == remote)
        .unwrap()
        .cursor()
        .target();
    let original_local = b_host
        .client()
        .get_session(local.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let original_remote = c_host
        .client()
        .get_session(remote.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let mut output = io::attach(&a_node, remote, None).await;
    let cursor = io::exchange(&a_node, remote, &mut output).await;
    drop((a_hold, b_hold, c_hold));
    let killed = b_node.crash().await;
    io::expect_transport_loss(&mut output, remote).await;
    drop(output);
    observe::connections(&a_node, &[]).await;
    observations::disconnected(&a_node, local_agent, local).await;
    observations::disconnected(&a_node, remote_agent, remote).await;
    observe::custody(&b_host, local, &original_local).await;
    observe::custody(&c_host, remote, &original_remote).await;
    let mut direct = io::attach(&c_node, remote, Some(cursor)).await;
    let direct_cursor = io::exchange(&c_node, remote, &mut direct).await;
    drop(direct);
    let b_node = RelayProcess::start(&b_host, &b_path, &b_socket, b, &config_path).await;
    assert!(!killed.matches());
    assert_eq!(a_node.wait_origins(&[b.hub(), c.hub()]).await, addresses);
    observations::same_agent(&a_node, local_agent, local).await;
    observations::same_agent(&a_node, remote_agent, remote).await;
    let mut recovered = io::attach(&a_node, remote, Some(direct_cursor)).await;
    io::exchange(&a_node, remote, &mut recovered).await;
    drop(recovered);
    let mut local_output = io::attach(&a_node, local, None).await;
    io::exchange(&a_node, local, &mut local_output).await;
    drop(local_output);
    observe::custody(&b_host, local, &original_local).await;
    observe::custody(&c_host, remote, &original_remote).await;
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}
