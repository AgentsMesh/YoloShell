use super::{
    federation_agents::launch, federation_support::*, management_support::ManagedNode,
    native_management_support,
};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_state::HubState;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    entry_hub: HubId,
    peer_hub: HubId,
    enrolled_revision: u64,
    conflicting_revision: u64,
    final_revision: u64,
    focus_unchanged: bool,
    agent_observed: bool,
    file_roots_managed: bool,
}

#[tokio::test(flavor = "multi_thread")]
async fn native_configuration_enrollment_and_conflict_preserve_remote_agent_without_focus() {
    const ENTRY: &str = "entry";
    const ORIGIN: &str = "origin";
    let a_host = crate::agent_recovery::federation_harness().await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let a_state = Arc::new(HubState::open(&root.path().join(ENTRY), a_host.home.path()).unwrap());
    let b_state = Arc::new(HubState::open(&root.path().join(ORIGIN), b_host.home.path()).unwrap());
    let (a, b) = (a_state.peer_identity(), b_state.peer_identity());
    let a_configuration = Configuration {
        ..Configuration::default()
    };
    let a_node = ManagedNode::start(&a_host, a_state, a_configuration).await;
    let b_configuration = Configuration {
        peers: vec![peer(a, None, false, vec![exports(b.hub(), false)])],
        trusted_origins: vec![],
        file_roots: vec![],
    };
    let b_node = RunningNode::start(&b_host, b_state.clone(), b_configuration).await;
    let target = launch(&b_node, &b_host, NAME_B).await;
    let before = b_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    native_management_support::run(
        &a_node,
        root.path(),
        peer(b, Some(&b_state), true, vec![]),
        target.address,
    )
    .await;
    let report: Report = serde_json::from_slice(
        &std::fs::read(root.path().join(native_management_support::REPORT_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(report.entry_hub, a.hub());
    assert_eq!(report.peer_hub, b.hub());
    assert!(
        report.enrolled_revision < report.conflicting_revision
            && report.conflicting_revision < report.final_revision
    );
    assert!(report.focus_unchanged && report.agent_observed && report.file_roots_managed);
    let current = a_node.snapshot().await;
    assert_eq!(report.final_revision, current.state.revision);
    assert!(current.state.configuration.peers.is_empty());
    assert!(current.state.configuration.file_roots.is_empty());
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
    a_node.stop().await;
    b_node.stop().await;
}
