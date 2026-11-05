use super::{
    federation_agents::launch, federation_support::*, management_support::ManagedNode,
    management_tui_driver::*,
};
use std::sync::Arc;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_enrolls_peer_rejects_concurrent_overwrite_and_revokes_without_agent_loss() {
    const A: &str = "entry";
    const B: &str = "origin";
    let a_host = crate::agent_recovery::federation_harness().await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let a_state = Arc::new(HubState::open(&root.path().join(A), a_host.home.path()).unwrap());
    let b_state = Arc::new(HubState::open(&root.path().join(B), b_host.home.path()).unwrap());
    let (a, b) = (a_state.peer_identity(), b_state.peer_identity());
    let config = Configuration {
        ..Configuration::default()
    };
    let a_node = ManagedNode::start(&a_host, a_state, config).await;
    let b_config = Configuration {
        peers: vec![peer(a, None, false, vec![exports(b.hub(), false)])],
        trusted_origins: vec![],
        file_roots: vec![],
    };
    let b_peer_socket = b_state.peer_socket().unwrap();
    let b_node = RunningNode::start(&b_host, b_state, b_config).await;
    let target = launch(&b_node, &b_host, NAME_B).await;
    let before = b_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let initial = a_node.snapshot().await;
    let mut driver = Driver::start(&a_host, &a_node).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(ADD_PEER)).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains("SSH Profile")).await;
    driver.field(&b.hub().to_string()).await;
    driver
        .field(
            &b.key()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        )
        .await;
    driver.field("in-process").await;
    driver.field(&b_peer_socket.display().to_string()).await;
    driver.send(TAB).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(DRAFT)).await;
    driver.root_apply().await;
    driver.active(initial.state.revision + 1).await;
    let configured = a_node.snapshot().await;
    assert_eq!(configured.state.configuration.peers[0].identity, b);
    assert!(configured.state.configuration.peers[0].consume);
    driver.send(ESC).await;
    driver.wait(|text| text.contains("Hubs:")).await;
    driver.send(AGENTS).await;
    driver.wait(|text| text.contains(SINGLE_AGENT_HEADER)).await;
    driver.send(DETAILS).await;
    driver
        .wait(|text| {
            detail_contains(text, &target.address.to_string()) && text.contains(HOOK_SOURCE)
        })
        .await;
    driver.send(ESC).await;
    driver.send(REOPEN_FROM_AGENTS).await;
    driver.active(configured.state.revision).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(ADD_PEER)).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(EDIT_PEER)).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains("SSH Profile")).await;
    driver.send(DOWN).await;
    driver.send(DOWN).await;
    driver.send(TOGGLE).await;
    driver.send(TAB).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(DRAFT)).await;
    let concurrent = a_node
        .configure(configured.state.configuration.clone())
        .await;
    driver.root_apply().await;
    driver.wait(|text| text.contains(CONFLICT)).await;
    driver.send(ENTER).await;
    assert_eq!(a_node.snapshot().await.state, concurrent.state);
    driver.send(DOWN).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(DISCARD)).await;
    driver.send(TAB).await;
    driver.send(ENTER).await;
    driver.active(concurrent.state.revision).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(ADD_PEER)).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(EDIT_PEER)).await;
    driver.send(DOWN).await;
    driver.send(DOWN).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains("Confirm")).await;
    driver.send(TAB).await;
    driver.send(ENTER).await;
    driver
        .wait(|text| text.contains(DRAFT) && text.contains("Connections (0)"))
        .await;
    driver.apply_from_root().await;
    driver.active(concurrent.state.revision + 1).await;
    assert!(a_node.snapshot().await.state.configuration.peers.is_empty());
    let after = b_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert_eq!(
        after.runtime.as_ref().unwrap().runtime_epoch,
        before.runtime.as_ref().unwrap().runtime_epoch
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_pid,
        before.agent.as_ref().unwrap().process_pid
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_start_time,
        before.agent.as_ref().unwrap().process_start_time
    );
    driver.finish().await;
    a_node.stop().await;
    b_node.stop().await;
}
