use super::{
    federation_agents::launch, federation_support::*, local_startup_support::retain_fixture_host,
};
use std::sync::Arc;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

#[path = "agent_control_canary.rs"]
mod canary;
#[path = "agent_control_checks.rs"]
mod checks;
#[path = "agent_control_custody.rs"]
mod custody;
#[path = "agent_control_durable.rs"]
mod durable;
#[path = "agent_control_policy.rs"]
mod policy;
#[path = "agent_control_requests.rs"]
mod requests;
use policy::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_multihop_native_agent_control_rejects_unsupported_effects_without_changing_custody() {
    exercise(Delegation::Control).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn real_multihop_agent_control_never_borrows_terminal_write_or_observe_rights() {
    exercise(Delegation::Attenuated).await;
}

async fn exercise(mode: Delegation) {
    let a_host = crate::agent_recovery::federation_harness().await;
    let a_hold = retain_fixture_host(&a_host).await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let b_hold = retain_fixture_host(&b_host).await;
    let c_host = crate::agent_recovery::federation_harness().await;
    let c_hold = retain_fixture_host(&c_host).await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let path = root.path().canonicalize().unwrap();
    let a_state =
        Arc::new(HubState::open(&path.join(ENTRY_DIRECTORY), a_host.home.path()).unwrap());
    let b_state =
        Arc::new(HubState::open(&path.join(RELAY_DIRECTORY), b_host.home.path()).unwrap());
    let c_state =
        Arc::new(HubState::open(&path.join(ORIGIN_DIRECTORY), c_host.home.path()).unwrap());
    let (a, b, c) = (
        a_state.peer_identity(),
        b_state.peer_identity(),
        c_state.peer_identity(),
    );
    let a_config = Configuration {
        trusted_origins: vec![c],
        peers: vec![peer(b, Some(&b_state), true, vec![])],
        file_roots: vec![],
    };
    let b_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![
            peer(a, None, false, vec![mode.to_entry(c.hub())]),
            peer(c, Some(&c_state), true, vec![]),
        ],
    };
    let c_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer(b, Some(&b_state), false, vec![mode.to_relay(c.hub())])],
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config).await;
    let b_node = RunningNode::start(&b_host, b_state, b_config).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config).await;
    drop((a_hold, b_hold, c_hold));
    let target = launch(&c_node, &c_host, AGENT_NAME).await;
    for node in [&a_node, &b_node, &c_node] {
        assert_eq!(node.wait_origins(&[c.hub()]).await.len(), SINGLE_AGENT);
    }
    checks::exercise(&a_node, &b_node, &c_node, &c_host, target, mode).await;
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}
