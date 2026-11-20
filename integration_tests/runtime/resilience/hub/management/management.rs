use super::{federation_agents::launch, federation_support::*, management_support::ManagedNode};
use std::sync::Arc;
use yolo_hub_agent_protocol::Reachability;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{AttachRequest, TerminalEvent};
use yolo_hub_state::HubState;
use yolo_hub_terminal_client::Attachment;
#[path = "management_diagnostics.rs"]
mod diagnostics;

#[tokio::test(flavor = "multi_thread")]
async fn online_relay_revocation_closes_old_streams_without_terminating_origin_agent() {
    diagnostics::initialize();
    let a_host = crate::agent_recovery::federation_harness().await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let c_host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let a_state = Arc::new(HubState::open(&root.path().join("a"), a_host.home.path()).unwrap());
    let b_state = Arc::new(HubState::open(&root.path().join("b"), b_host.home.path()).unwrap());
    let c_state = Arc::new(HubState::open(&root.path().join("c"), c_host.home.path()).unwrap());
    let (a, b, c) = (
        a_state.peer_identity(),
        b_state.peer_identity(),
        c_state.peer_identity(),
    );
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
    let b_node = ManagedNode::start(&b_host, b_state, b_config.clone()).await;
    let a_config = Configuration {
        trusted_origins: vec![c],
        file_roots: vec![],
        peers: vec![peer_at_socket(b, b_node.peer_socket.clone(), true, vec![])],
    };
    let c_config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer_at_socket(
            b,
            b_node.peer_socket.clone(),
            false,
            vec![exports(c.hub(), true)],
        )],
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config).await;
    let target = launch(&c_node, &c_host, NAME_C).await;
    let identities = a_node.wait_origins(&[c.hub()]).await;
    let before = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let mut output = Attachment::open(
        a_node.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let mut revoked = b_config.clone();
    revoked
        .peers
        .iter_mut()
        .find(|peer| peer.identity == a)
        .unwrap()
        .exports
        .retain(|policy| policy.origin != c.hub());
    let receipt = b_node.configure(revoked).await;
    assert_eq!(receipt.peer_socket.as_ref(), Some(&b_node.peer_socket));
    tokio::time::timeout_at(deadline(), async {
        loop {
            match output.next_event(deadline()).await {
                Err(_) | Ok(TerminalEvent::Unreachable { .. }) => break,
                Ok(TerminalEvent::Exited { .. }) => panic!("relay revocation reported origin exit"),
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    tokio::time::timeout_at(deadline(), async {
        loop {
            let agents = a_node.agents().await;
            assert_eq!(
                agents
                    .iter()
                    .map(|agent| agent.cursor().target().address)
                    .collect::<std::collections::BTreeSet<_>>(),
                identities
            );
            if agents
                .iter()
                .all(|agent| agent.fields().reachability == Reachability::Unreachable)
            {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    let after = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert_eq!(
        after.runtime.as_ref().unwrap().runtime_epoch,
        target.runtime_epoch.as_uuid()
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_pid,
        before.agent.as_ref().unwrap().process_pid
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_start_time,
        before.agent.as_ref().unwrap().process_start_time
    );
    assert!(
        b_node
            .client()
            .await
            .call(c.hub(), Command::GetConfiguration {}, deadline())
            .await
            .is_err()
    );
    let restored = b_node.configure(b_config).await;
    assert_eq!(restored.peer_socket.as_ref(), Some(&b_node.peer_socket));
    assert_eq!(a_node.wait_origins(&[c.hub()]).await, identities);
    let replay_client = a_node.client().await;
    let replay_request = AttachRequest::new(target, None).unwrap();
    let replay_deadline = deadline();
    let replay = match Attachment::open(replay_client, replay_request, replay_deadline).await {
        Ok(replay) => replay,
        Err(error) => diagnostics::fail(&a_node, &c_node, target, &error, replay_deadline).await,
    };
    assert_eq!(replay.cursor().target, target);
    drop(replay);
    drop(output);
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}
