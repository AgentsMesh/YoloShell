use super::{federation_agents::launch_queryable, federation_support::*};
use std::sync::Arc;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_routing::Right;
use yolo_hub_state::HubState;

#[path = "federation_observe_denials.rs"]
mod denials;

const ENTRY_DIRECTORY: &str = "observe-entry";
const RELAY_DIRECTORY: &str = "observe-relay";
const ORIGIN_DIRECTORY: &str = "observe-origin";
const AGENT_NAME: &str = "observe-only-origin-agent";

#[tokio::test(flavor = "multi_thread")]
async fn real_three_hubs_attenuate_origin_observe_only_grant_without_granting_control() {
    let a_host = crate::agent_recovery::federation_harness().await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let c_host = crate::agent_recovery::federation_harness().await;
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
            peer(
                a,
                None,
                false,
                vec![exports(b.hub(), false), exports(c.hub(), false)],
            ),
            peer(c, None, true, vec![]),
        ],
    };
    let mut origin_policy = exports(c.hub(), true);
    origin_policy.rights = [Right::Observe].into_iter().collect();
    let c_config = Configuration {
        listen: Some(c_addr),
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer(b, Some(b_addr), false, vec![origin_policy])],
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config, a_listener).await;
    let b_node = RunningNode::start(&b_host, b_state, b_config, b_listener).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config, c_listener).await;
    let created = launch_queryable(&c_node, &c_host, AGENT_NAME).await;
    let from_a = a_node.wait_origins(&[c.hub()]).await;
    let from_b = b_node.wait_origins(&[c.hub()]).await;
    let from_c = c_node.wait_origins(&[c.hub()]).await;
    assert_eq!(from_a, from_c);
    assert_eq!(from_b, from_c);
    let address = *from_c.first().unwrap();
    denials::verify(&a_node, &b_node, &c_node, &c_host, &created, address).await;
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}
