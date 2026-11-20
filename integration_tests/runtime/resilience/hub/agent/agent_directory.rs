use super::federation_support::*;
use super::local_startup_support::retain_fixture_host;
use std::sync::Arc;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

#[path = "agent_directory_constants.rs"]
mod constants;
#[path = "agent_directory_launch.rs"]
mod launch;
#[path = "agent_directory_observe.rs"]
mod observe;
use constants::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_hook_working_directory_changes_keep_agent_identity_across_relay_and_disconnect() {
    let a_host = crate::agent_recovery::federation_harness().await;
    let a_hold = retain_fixture_host(&a_host).await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let b_hold = retain_fixture_host(&b_host).await;
    let c_host = crate::agent_recovery::federation_harness().await;
    let c_hold = retain_fixture_host(&c_host).await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let path = root.path().canonicalize().unwrap();
    let a_state = Arc::new(HubState::open(&path.join(A_STATE), a_host.home.path()).unwrap());
    let b_state = Arc::new(HubState::open(&path.join(B_STATE), b_host.home.path()).unwrap());
    let c_state = Arc::new(HubState::open(&path.join(C_STATE), c_host.home.path()).unwrap());
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
            peer(a, None, false, vec![exports(c.hub(), false)]),
            peer(c, Some(&c_state), true, vec![]),
        ],
    };
    let c_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer(b, Some(&b_state), false, vec![exports(c.hub(), true)])],
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config).await;
    let b_node = RunningNode::start(&b_host, b_state, b_config).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config).await;
    drop((a_hold, b_hold, c_hold));
    let (binary, first_directory, next_directory) = launch::prepare(&c_host);
    let target = launch::launch(&c_node, &c_host, &binary, &first_directory).await;
    let before = observe::current(&c_node, target, &first_directory, None).await;
    let before_entry = observe::current(&a_node, target, &first_directory, None).await;
    observe::identities(&before, &before_entry);
    observe::exact_query(&a_node, &before).await;
    let original = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert_ne!(
        original.manifest.cwd.canonicalize().unwrap(),
        first_directory
    );
    assert_eq!(
        original.agent.as_ref().unwrap().cwd,
        first_directory.to_str().unwrap()
    );
    observe::no_file_grant(&a_node, &b_node, &c_node).await;
    // Input reaches the same live Claude fixture: chdir, transcript append, native CwdChanged.
    launch::change(&a_node, target, &next_directory).await;
    let changed = observe::current(&c_node, target, &next_directory, Some(&before)).await;
    let changed_entry =
        observe::current(&a_node, target, &next_directory, Some(&before_entry)).await;
    let changed_relay = observe::current(&b_node, target, &next_directory, None).await;
    observe::identities(&changed, &changed_entry);
    observe::identities(&changed, &changed_relay);
    observe::exact_query(&a_node, &changed).await;
    observe::runtime(
        &c_host,
        target,
        changed.cursor().target(),
        &next_directory,
        &original,
    )
    .await;
    observe::no_file_grant(&a_node, &b_node, &c_node).await;
    b_node.stop().await;
    observe::stale(&a_node, &changed_entry).await;
    let surviving = observe::current(&c_node, target, &next_directory, None).await;
    observe::identities(&changed, &surviving);
    observe::runtime(
        &c_host,
        target,
        surviving.cursor().target(),
        &next_directory,
        &original,
    )
    .await;
    a_node.stop().await;
    c_node.stop().await;
}
