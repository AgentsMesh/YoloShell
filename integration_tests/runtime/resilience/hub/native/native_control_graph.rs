use super::{
    super::{federation_support::*, local_startup_support::retain_fixture_host},
    constants::*,
    flow,
};
use std::sync::Arc;
use yolo_hub_federation_settings::{Configuration, ExportPolicy};
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;
use yolo_hub_state::HubState;

pub async fn run() {
    let a_host = crate::agent_recovery::federation_harness().await;
    let a_hold = retain_fixture_host(&a_host).await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let b_hold = retain_fixture_host(&b_host).await;
    let c_host = crate::agent_recovery::federation_harness().await;
    let c_hold = retain_fixture_host(&c_host).await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let path = root.path().canonicalize().unwrap();
    let a_state = Arc::new(HubState::open(&path.join(ENTRY), a_host.home.path()).unwrap());
    let b_state = Arc::new(HubState::open(&path.join(RELAY), b_host.home.path()).unwrap());
    let c_state = Arc::new(HubState::open(&path.join(ORIGIN), c_host.home.path()).unwrap());
    let (a, b, c) = (
        a_state.peer_identity(),
        b_state.peer_identity(),
        c_state.peer_identity(),
    );
    let (a_listener, b_listener, c_listener) =
        (listener().await, listener().await, listener().await);
    let (a_addr, b_addr, c_addr) = (
        a_listener.local_addr().unwrap(),
        b_listener.local_addr().unwrap(),
        c_listener.local_addr().unwrap(),
    );
    let a_config = Configuration {
        listen: Some(a_addr),
        trusted_origins: vec![c],
        file_roots: vec![],
        peers: vec![peer(b, Some(b_addr), true, vec![])],
    };
    let b_config = Configuration {
        listen: Some(b_addr),
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![
            peer(a, None, false, vec![control_exports(c.hub(), false)]),
            peer(c, None, true, vec![]),
        ],
    };
    let c_config = Configuration {
        listen: Some(c_addr),
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer(
            b,
            Some(b_addr),
            false,
            vec![control_exports(c.hub(), true)],
        )],
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config, a_listener).await;
    let b_node = RunningNode::start(&b_host, b_state, b_config, b_listener).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config, c_listener).await;
    drop((a_hold, b_hold, c_hold));
    flow::exercise(&a_node, &b_node, &c_node, &c_host).await;
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}

fn control_exports(origin: HubId, relay: bool) -> ExportPolicy {
    let mut policy = exports(origin, relay);
    policy.rights = policy
        .rights
        .union(&[Right::AgentControl].into_iter().collect());
    policy
}
