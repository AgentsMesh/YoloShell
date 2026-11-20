use super::{federation_agents::launch, federation_support::*};
use std::sync::Arc;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

#[path = "multipath_constants.rs"]
mod constants;
#[path = "multipath_io.rs"]
pub(super) mod io;
#[path = "multipath_observe.rs"]
pub(super) mod observe;
#[path = "multipath_proxy.rs"]
mod proxy;
use constants::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_direct_and_relay_paths_deduplicate_and_fail_over_without_replacing_runtime() {
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
    let proxy = proxy::RouteProxy::start(c_state.peer_socket().unwrap()).await;
    let a_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![
            peer(b, Some(&b_state), true, vec![]),
            peer_at_socket(c, proxy.path.clone(), true, vec![]),
        ],
    };
    let b_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![
            peer(a, None, false, vec![exports(c.hub(), false)]),
            peer(c, Some(&c_state), true, vec![]),
        ],
    };
    let c_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![
            peer(a, None, false, vec![exports(c.hub(), false)]),
            peer(b, None, false, vec![exports(c.hub(), true)]),
        ],
    };
    let c_node = RunningNode::start(&c_host, c_state, c_config).await;
    let a_node = RunningNode::start(&a_host, a_state, a_config).await;
    let target = launch(&c_node, &c_host, AGENT_NAME).await;
    let expected = c_node.wait_origins(&[c.hub()]).await;
    assert_eq!(a_node.wait_origins(&[c.hub()]).await, expected);
    let agent = *expected.first().unwrap();
    let original = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    // B is not running yet, so this channel and exchange prove the direct path.
    let mut direct = io::attach(&a_node, target, None).await;
    let cursor = io::exchange(&a_node, target, &mut direct).await;
    let b_node = RunningNode::start(&b_host, b_state, b_config).await;
    assert_eq!(b_node.wait_origins(&[c.hub()]).await, expected);
    observe::connections(&a_node, &[b.hub(), c.hub()]).await;
    observe::stable_inventory(&a_node, agent, target).await;
    // Only the owned A-to-C TCP forwarding sockets are cut; C keeps its PTY.
    proxy.cut().await;
    io::expect_transport_loss(&mut direct, target).await;
    drop(direct);
    observe::connections(&a_node, &[b.hub()]).await;
    assert_eq!(a_node.wait_origins(&[c.hub()]).await, expected);
    let mut relayed = io::attach(&a_node, target, Some(cursor)).await;
    let relayed_cursor = io::exchange(&a_node, target, &mut relayed).await;
    observe::custody(&c_host, target, &original).await;
    drop(relayed);
    proxy.restore();
    observe::connections(&a_node, &[b.hub(), c.hub()]).await;
    observe::stable_inventory(&a_node, agent, target).await;
    // Orderly B shutdown removes the other route; direct C remains sufficient.
    b_node.stop().await;
    observe::connections(&a_node, &[c.hub()]).await;
    assert_eq!(a_node.wait_origins(&[c.hub()]).await, expected);
    let mut recovered = io::attach(&a_node, target, Some(relayed_cursor)).await;
    io::exchange(&a_node, target, &mut recovered).await;
    observe::custody(&c_host, target, &original).await;
    drop(recovered);
    a_node.stop().await;
    proxy.stop().await;
    c_node.stop().await;
}
