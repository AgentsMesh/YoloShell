#[path = "public_client_multihop_catalog.rs"]
mod catalog;
#[path = "public_client_multihop_setup.rs"]
mod setup;

use super::multihop_ingress::{Ingress, SharedClient};
use crate::agent_recovery::federation_harness;
use crate::hub::{
    federation_agents::{launch, launch_queryable},
    federation_support::{self, RunningNode},
};
use std::sync::Arc;
use yolo_hub_client::HubEndpoint;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::AttachRequest;
use yolo_hub_state::HubState;
use yolo_hub_terminal_client::Attachment;

#[tokio::test(flavor = "multi_thread")]
async fn openssh_entries_preserve_directed_views_and_control_c_origin() {
    let a_host = federation_harness().await;
    let b_host = federation_harness().await;
    let c_host = federation_harness().await;
    let root = tempfile::tempdir().unwrap();
    let a_state = Arc::new(HubState::open(&root.path().join("a"), a_host.home.path()).unwrap());
    let b_state = Arc::new(HubState::open(&root.path().join("b"), b_host.home.path()).unwrap());
    let c_state = Arc::new(HubState::open(&root.path().join("c"), c_host.home.path()).unwrap());
    let (a_peer, b_peer, c_peer) = (
        a_state.peer_identity(),
        b_state.peer_identity(),
        c_state.peer_identity(),
    );
    let sockets = (
        a_state.peer_socket().unwrap(),
        b_state.peer_socket().unwrap(),
        c_state.peer_socket().unwrap(),
    );
    let (a_config, b_config, c_config) = setup::configuration(a_peer, b_peer, c_peer, sockets);
    let a_node = RunningNode::start(&a_host, a_state.clone(), a_config).await;
    let b_node = RunningNode::start(&b_host, b_state.clone(), b_config).await;
    let c_node = RunningNode::start(&c_host, c_state.clone(), c_config).await;
    let shared = SharedClient::generate(root.path());
    let a_ingress = Ingress::start(
        &a_node,
        a_state,
        &shared,
        &[a_peer.hub(), b_peer.hub(), c_peer.hub()],
    )
    .await;
    let b_ingress = Ingress::start(&b_node, b_state, &shared, &[b_peer.hub(), c_peer.hub()]).await;
    let c_ingress = Ingress::start(&c_node, c_state, &shared, &[c_peer.hub()]).await;
    let a_agent = launch(&a_node, &a_host, super::constants::MULTIHOP_AGENT_A).await;
    let b_agent = launch(&b_node, &b_host, super::constants::MULTIHOP_AGENT_B).await;
    let c_agent = launch_queryable(&c_node, &c_host, super::constants::MULTIHOP_AGENT_C).await;
    a_node
        .wait_origins(&[a_peer.hub(), b_peer.hub(), c_peer.hub()])
        .await;
    b_node.wait_origins(&[b_peer.hub(), c_peer.hub()]).await;
    c_node.wait_origins(&[c_peer.hub()]).await;
    let expected_agents = [
        catalog::terminal_agent(&a_node, a_agent).await,
        catalog::terminal_agent(&b_node, b_agent).await,
        catalog::terminal_agent(&c_node, c_agent.target).await,
    ];
    let mut a = HubEndpoint::remote(a_ingress.entry.clone())
        .connect(a_peer.hub(), federation_support::deadline())
        .await
        .unwrap();
    let mut b = HubEndpoint::remote(b_ingress.entry.clone())
        .connect(b_peer.hub(), federation_support::deadline())
        .await
        .unwrap();
    let mut c = HubEndpoint::remote(c_ingress.entry.clone())
        .connect(c_peer.hub(), federation_support::deadline())
        .await
        .unwrap();
    catalog::assert_views(
        &mut a,
        &mut b,
        &mut c,
        a_peer.hub(),
        b_peer.hub(),
        c_peer.hub(),
        expected_agents,
    )
    .await;
    let tui_host = crate::Harness::start().await;
    let tui_root = tempfile::tempdir().unwrap();
    super::multihop_tui::observe_entry(
        &tui_host,
        &a_ingress,
        &shared,
        a_peer.hub(),
        tui_root.path(),
        &[
            super::constants::MULTIHOP_AGENT_A,
            super::constants::MULTIHOP_AGENT_B,
            super::constants::MULTIHOP_AGENT_C,
        ],
        &[],
    )
    .await;
    let tui_root = tempfile::tempdir().unwrap();
    super::multihop_tui::observe_entry(
        &tui_host,
        &b_ingress,
        &shared,
        b_peer.hub(),
        tui_root.path(),
        &[
            super::constants::MULTIHOP_AGENT_B,
            super::constants::MULTIHOP_AGENT_C,
        ],
        &[super::constants::MULTIHOP_AGENT_A],
    )
    .await;
    let tui_root = tempfile::tempdir().unwrap();
    super::multihop_tui::observe_entry(
        &tui_host,
        &c_ingress,
        &shared,
        c_peer.hub(),
        tui_root.path(),
        &[super::constants::MULTIHOP_AGENT_C],
        &[
            super::constants::MULTIHOP_AGENT_A,
            super::constants::MULTIHOP_AGENT_B,
        ],
    )
    .await;
    let (plain, target) = setup::create_shell(&mut a, c_peer.hub(), &c_host).await;
    let output_client = HubEndpoint::remote(a_ingress.entry.clone())
        .connect(a_peer.hub(), federation_support::deadline())
        .await
        .unwrap();
    let mut output = Attachment::open(
        output_client,
        AttachRequest::new(target, None).unwrap(),
        federation_support::deadline(),
    )
    .await
    .unwrap();
    let grant = setup::acquire(&mut a, target, shared.identity.client_id).await;
    let (command, marker) = setup::marker();
    a.call(
        c_peer.hub(),
        setup::input(grant.clone(), &command),
        federation_support::deadline(),
    )
    .await
    .unwrap();
    let bytes = super::output::until(&mut output, &marker).await;
    assert!(String::from_utf8_lossy(&bytes).contains(&marker));
    a.call(
        c_peer.hub(),
        Command::Release { grant },
        federation_support::deadline(),
    )
    .await
    .unwrap();
    catalog::assert_shell_visible(
        &mut a,
        &mut b,
        &mut c,
        plain,
        a_peer.hub(),
        b_peer.hub(),
        c_peer.hub(),
    )
    .await;
    drop((output, a, b, c));
    c_ingress.close().await;
    b_ingress.close().await;
    a_ingress.close().await;
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}
