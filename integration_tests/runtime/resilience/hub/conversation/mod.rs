use super::{federation_agents::launch, federation_support::*, management_support::ManagedNode};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use yolo_conversation_protocol::{ConversationPart, ConversationReadResult};
use yolo_hub_client::ClientError;
use yolo_hub_conversation_client::{self as conversation, ConversationClientError};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_state::HubState;

mod constants;
mod support;
use constants::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_three_hub_conversation_reads_preserve_origin_and_revocation_keeps_the_pty_alive() {
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
    let mut b_config = Configuration {
        listen: Some(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), DYNAMIC_PORT)),
        peers: vec![
            peer(a, None, false, vec![support::export(c.hub(), false)]),
            peer(c, None, true, vec![]),
        ],
        ..Configuration::default()
    };
    let b_node = ManagedNode::start(&b_host, b_state, b_config.clone()).await;
    let a_listener = listener().await;
    let c_listener = listener().await;
    let a_config = Configuration {
        listen: Some(a_listener.local_addr().unwrap()),
        peers: vec![peer(b, Some(b_node.address), true, vec![])],
        trusted_origins: vec![c],
        file_roots: vec![],
    };
    let c_config = Configuration {
        listen: Some(c_listener.local_addr().unwrap()),
        peers: vec![peer(
            b,
            Some(b_node.address),
            false,
            vec![support::export(c.hub(), true)],
        )],
        ..Configuration::default()
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config, a_listener).await;
    let c_node = RunningNode::start(&c_host, c_state, c_config, c_listener).await;
    let target = launch(&c_node, &c_host, NAME_C).await;
    a_node.wait_origins(&[c.hub()]).await;
    let before = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    // This fake agent's native rollout may contain metadata only. Append content
    // exclusively to the isolated fixture file, after verifying its private root.
    support::append_fixture_content(&c_host, &before);
    let agents = c_node.agents().await;
    let [agent] = agents.as_slice() else {
        panic!("expected one origin agent")
    };
    let descriptor = support::resolve(&a_node, agent.cursor().target().address).await;
    assert_eq!(descriptor.address.hub_id, c.hub());
    assert_eq!(descriptor.binding.agent, agent.cursor().target());
    assert_eq!(descriptor.binding.runtime, target);
    let request = support::request(&descriptor);
    let reading = conversation::read(&mut a_node.client().await, request.clone(), deadline())
        .await
        .unwrap();
    reading.validate_for(&request).unwrap();
    let ConversationReadResult::Page { page } = reading else {
        panic!("expected native content")
    };
    assert!(
        page.entries
            .iter()
            .flat_map(|entry| &entry.parts)
            .any(|part| matches!(part, ConversationPart::Text { text } if text == CONTENT_MARKER))
    );

    b_config
        .peers
        .iter_mut()
        .find(|peer| peer.identity == a)
        .unwrap()
        .exports = vec![exports(c.hub(), false)];
    b_node.configure(b_config).await;
    tokio::time::timeout_at(deadline(), async {
        loop {
            if matches!(
                conversation::read(&mut a_node.client().await, request.clone(), deadline()).await,
                Err(ConversationClientError::Client(ClientError::Remote(
                    PublicError::Unreachable | PublicError::Unauthorized
                )))
            ) {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    a_node.wait_origins(&[c.hub()]).await;
    super::files_support::visible_session(&a_node, target).await;
    // A real terminal input/output exchange must still succeed through the same relay.
    super::attach_federation::exercise(&a_node, target).await;
    let after = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let first = before.runtime.as_ref().unwrap();
    let last = after.runtime.as_ref().unwrap();
    assert_eq!(
        (
            first.runtime_epoch,
            first.worker_pid,
            first.worker_start_time,
            first.child_pid,
            first.child_start_time
        ),
        (
            last.runtime_epoch,
            last.worker_pid,
            last.worker_start_time,
            last.child_pid,
            last.child_start_time
        )
    );
    assert_eq!(
        (
            before.agent.as_ref().unwrap().process_pid,
            before.agent.as_ref().unwrap().process_start_time
        ),
        (
            after.agent.as_ref().unwrap().process_pid,
            after.agent.as_ref().unwrap().process_start_time
        )
    );
    assert!(matches!(
        conversation::read(&mut a_node.client().await, request.clone(), deadline()).await,
        Err(ConversationClientError::Client(ClientError::Remote(
            PublicError::Unreachable | PublicError::Unauthorized
        )))
    ));
    assert!(
        conversation::read(&mut c_node.client().await, request, deadline())
            .await
            .is_ok()
    );
    a_node.stop().await;
    b_node.stop().await;
    c_node.stop().await;
}
