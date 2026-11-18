use super::{
    federation_agents::*,
    federation_creation_query as creation_query,
    federation_support::*,
    support::{acquire, input},
};
use std::sync::Arc;
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, LeaseMode, TerminalEvent};
use yolo_hub_state::HubState;
use yolo_hub_terminal_client::Attachment;

#[tokio::test(flavor = "multi_thread")]
async fn real_three_hubs_relay_reverse_connection_and_tui_keep_origin_custody() {
    let a_host = crate::agent_recovery::federation_harness().await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let c_host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let root = root.path().canonicalize().unwrap();
    let a_state = Arc::new(HubState::open(&root.join("a"), a_host.home.path()).unwrap());
    let b_path = root.join("b");
    let b_state = Arc::new(HubState::open(&b_path, b_host.home.path()).unwrap());
    let c_state = Arc::new(HubState::open(&root.join("c"), c_host.home.path()).unwrap());
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
    let c_config = Configuration {
        listen: Some(c_addr),
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer(b, Some(b_addr), false, vec![exports(c.hub(), true)])],
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config, a_listener).await;
    let b_node = RunningNode::start(&b_host, b_state, b_config.clone(), b_listener).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config, c_listener).await;
    launch(&a_node, &a_host, NAME_A).await;
    launch(&b_node, &b_host, NAME_B).await;
    let created = launch_queryable(&c_node, &c_host, NAME_C).await;
    let target = created.target;
    let all = a_node.wait_origins(&[a.hub(), b.hub(), c.hub()]).await;
    let bc = b_node.wait_origins(&[b.hub(), c.hub()]).await;
    let only_c = c_node.wait_origins(&[c.hub()]).await;
    assert!(bc.is_subset(&all) && only_c.is_subset(&bc));
    creation_query::verify(&[&a_node, &b_node, &c_node], &created).await;
    super::federation_hubs::verify(&a_node, &b_node, &c_node).await;
    super::catalog_federation::verify(&a_node, &b_node, &c_node).await;
    let before = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let mut client = a_node.client().await;
    let Reply::Lease { grant } = client
        .call(c.hub(), acquire(target, LeaseMode::Acquire {}), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let mut output = Attachment::open(
        a_node.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let (command, marker) = ping();
    client
        .call(c.hub(), input(grant, &command), deadline())
        .await
        .unwrap();
    let mut observed = output
        .initial()
        .fields()
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.fields().data.clone())
        .unwrap_or_default();
    let echoed = tokio::time::timeout_at(deadline(), async {
        while !String::from_utf8_lossy(&observed).contains(&marker) {
            if let TerminalEvent::Output { chunk } = output.next_event(deadline()).await.unwrap() {
                observed.extend_from_slice(&chunk.fields().data);
            }
        }
    })
    .await;
    assert!(
        echoed.is_ok(),
        "direct agent input did not complete: {}",
        String::from_utf8_lossy(&observed)
    );
    let cursor = output.cursor();
    client
        .call(c.hub(), Command::Release { grant }, deadline())
        .await
        .unwrap();
    drop(output);
    super::attach_federation::exercise(&a_node, target).await;
    b_node.stop().await;
    tokio::time::timeout_at(deadline(), async {
        loop {
            let agents = a_node.agents().await;
            let retained = agents
                .iter()
                .filter(|agent| agent.cursor().target().address.hub_id == c.hub())
                .collect::<Vec<_>>();
            assert_eq!(retained.len(), only_c.len());
            assert_eq!(
                retained
                    .iter()
                    .map(|agent| agent.cursor().target().address)
                    .collect::<std::collections::BTreeSet<_>>(),
                only_c
            );
            if retained.iter().all(|agent| {
                agent.fields().reachability == Reachability::Unreachable
                    && agent.fields().freshness == Freshness::Stale
            }) {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    creation_query::relay_lost(&a_node, &c_node, &[&a_host, &b_host, &c_host], &created).await;
    let surviving = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert_eq!(
        surviving.runtime.as_ref().unwrap().runtime_epoch,
        target.runtime_epoch.as_uuid()
    );
    assert_eq!(
        surviving.agent.as_ref().unwrap().process_pid,
        before.agent.as_ref().unwrap().process_pid
    );
    assert_eq!(
        surviving.agent.as_ref().unwrap().process_start_time,
        before.agent.as_ref().unwrap().process_start_time
    );
    let listener = tokio::net::TcpListener::bind(b_addr).await.unwrap();
    let b_state = Arc::new(HubState::open(&b_path, b_host.home.path()).unwrap());
    assert_eq!(b_state.peer_identity(), b);
    let b_node = RunningNode::start(&b_host, b_state, b_config, listener).await;
    assert_eq!(a_node.wait_origins(&[a.hub(), b.hub(), c.hub()]).await, all);
    creation_query::verify(&[&a_node], &created).await;
    let replay = Attachment::open(
        a_node.client().await,
        AttachRequest::new(target, Some(cursor)).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    assert_eq!(replay.cursor().target, target);
    drop(replay);
    let all_names = [NAME_A, NAME_B, NAME_C];
    super::federation_tui::exercise(&a_node, &a_host, &all_names, &[], Some(&c_host)).await;
    super::federation_tui::exercise(&b_node, &b_host, &[NAME_B, NAME_C], &[NAME_A], None).await;
    super::federation_tui::exercise(&c_node, &c_host, &[NAME_C], &[NAME_A, NAME_B], None).await;
    drop(client);
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}
