use super::{
    federation_agents::launch, federation_support::*, files_constants::*, files_support::*,
    management_support::ManagedNode,
};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_file_protocol::ReadFileRequest;
use yolo_hub_identity::FileRootId;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_state::HubState;

#[tokio::test(flavor = "multi_thread")]
async fn real_multihop_files_require_independent_rights_and_preserve_origin_pty_custody() {
    let a_host = crate::agent_recovery::federation_harness().await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let c_host = crate::agent_recovery::federation_harness().await;
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
    let shared = FileRootId::new();
    let a_root = configure_root(&path, A_FILES, shared, A_CONTENT);
    let b_root = configure_root(&path, B_FILES, shared, B_CONTENT);
    let c_root = configure_root(&path, C_FILES, shared, C_CONTENT);
    let mut b_config = Configuration {
        peers: vec![
            peer(
                a,
                None,
                false,
                vec![file_export(b.hub(), false), exports(c.hub(), false)],
            ),
            peer(c, Some(&c_state), true, vec![]),
        ],
        file_roots: vec![b_root],
        ..Configuration::default()
    };
    let b_node = ManagedNode::start(&b_host, b_state, b_config.clone()).await;
    let a_config = Configuration {
        peers: vec![peer_at_socket(b, b_node.peer_socket.clone(), true, vec![])],
        trusted_origins: vec![c],
        file_roots: vec![a_root],
    };
    let c_config = Configuration {
        peers: vec![peer_at_socket(
            b,
            b_node.peer_socket.clone(),
            false,
            vec![file_export(c.hub(), true)],
        )],
        file_roots: vec![c_root],
        ..Configuration::default()
    };
    let a_node = RunningNode::start(&a_host, a_state, a_config).await;
    let c_node = RunningNode::start(&c_host, c_state.clone(), c_config.clone()).await;
    let target = launch(&c_node, &c_host, NAME_C).await;
    a_node.wait_origins(&[c.hub()]).await;
    let before = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    visible_session(&a_node, target).await;
    assert!(
        a_node
            .client()
            .await
            .call(c.hub(), Command::ListFileRoots { after: None }, deadline())
            .await
            .is_err()
    );
    let a_roots = roots(&a_node, a.hub()).await;
    assert_eq!(a_roots.roots.len(), 1);
    assert_eq!(
        bytes(&a_node, &a_roots.roots[0]).await.fields().data,
        A_CONTENT
    );
    let b_roots = roots(&a_node, b.hub()).await;
    assert_eq!(
        bytes(&a_node, &b_roots.roots[0]).await.fields().data,
        B_CONTENT
    );
    enable_files(&mut b_config, a.hub(), c.hub(), true);
    b_node.configure(b_config.clone()).await;
    let c_roots = roots(&a_node, c.hub()).await;
    let original = c_roots.roots[0].clone();
    assert_eq!(original.fields().target.address.root_id, shared);
    assert_eq!(original.fields().target.address.hub_id, c.hub());
    let content = bytes(&a_node, &original).await;
    assert_eq!(content.fields().data, C_CONTENT);
    protected_preflight_keeps_live_configuration(&b_node, b_host.home.path()).await;
    assert_eq!(bytes(&a_node, &original).await.fields().data, C_CONTENT);
    std::fs::rename(path.join(C_FILES), path.join(OLD_FILES)).unwrap();
    std::fs::create_dir(path.join(C_FILES)).unwrap();
    std::fs::write(path.join(C_FILES).join(RELATIVE_FILE), REPLACED_CONTENT).unwrap();
    assert!(matches!(
        yolo_hub_file_client::read(&mut a_node.client().await, request(&original), deadline())
            .await,
        Err(yolo_hub_file_client::FileClientError::Client(
            yolo_hub_client::ClientError::Remote(PublicError::Conflict)
        ))
    ));
    c_node.stop().await;
    let c_node = RunningNode::start(&c_host, c_state, c_config).await;
    a_node.wait_origins(&[c.hub()]).await;
    let reopened = roots(&a_node, c.hub()).await.roots.remove(0);
    assert_ne!(
        reopened.fields().target.epoch,
        original.fields().target.epoch
    );
    assert!(matches!(
        yolo_hub_file_client::read(&mut a_node.client().await, request(&original), deadline())
            .await,
        Err(yolo_hub_file_client::FileClientError::Client(
            yolo_hub_client::ClientError::Remote(PublicError::TargetMismatch)
        ))
    ));
    let mut stale = request(&reopened).fields().clone();
    stale.expected_version = Some(content.fields().version);
    assert!(
        yolo_hub_file_client::read(
            &mut a_node.client().await,
            ReadFileRequest::new(stale).unwrap(),
            deadline()
        )
        .await
        .is_err()
    );
    assert_eq!(
        bytes(&a_node, &reopened).await.fields().data,
        REPLACED_CONTENT
    );
    enable_files(&mut b_config, a.hub(), c.hub(), false);
    b_node.configure(b_config).await;
    tokio::time::timeout_at(deadline(), async {
        loop {
            if yolo_hub_file_client::read(
                &mut a_node.client().await,
                request(&reopened),
                deadline(),
            )
            .await
            .is_err()
            {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    a_node.wait_origins(&[c.hub()]).await;
    visible_session(&a_node, target).await;
    let after = c_host
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
    c_node.stop().await;
}
