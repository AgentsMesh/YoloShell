use super::{
    federation_agents::launch_managed, federation_support::*, files_constants::*,
    files_gate_support::*, files_support::*, management_support::ManagedNode,
};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_file_executor_test_support::constants::READ_RELEASE_FILE;
use yolo_hub_identity::FileRootId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_state::HubState;

#[tokio::test(flavor = "multi_thread")]
async fn revoking_at_the_origin_or_relay_withholds_in_flight_file_bytes_without_killing_agent() {
    for at_origin in [false, true] {
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
        let b_config = Configuration {
            peers: vec![
                peer(a, None, false, vec![file_export(c.hub(), false)]),
                peer(c, Some(&c_state), true, vec![]),
            ],
            ..Configuration::default()
        };
        let b_node = ManagedNode::start(&b_host, b_state, b_config.clone()).await;
        let c_root = configure_root(&path, C_FILES, FileRootId::new(), C_CONTENT);
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
        let c_node = ManagedNode::start_with_helper(
            &c_host,
            c_state,
            c_config.clone(),
            gated_program(&path),
        )
        .await;
        let a_config = Configuration {
            peers: vec![peer_at_socket(b, b_node.peer_socket.clone(), true, vec![])],
            trusted_origins: vec![c],
            file_roots: vec![],
        };
        let a_node = ManagedNode::start(&a_host, a_state, a_config).await;
        let target = launch_managed(&c_node, &c_host, NAME_C).await;
        let before = c_host
            .client()
            .get_session(target.address.terminal_id.as_uuid())
            .await
            .unwrap();
        let scope = wait_roots(&a_node, c.hub()).await.roots.remove(0);
        let mut client = a_node.client().await;
        let request = request(&scope);
        let reading = tokio::spawn(async move {
            yolo_hub_file_client::read(&mut client, request, deadline()).await
        });
        let proof = wait_read(&path.join(C_FILES)).await;
        assert!(!proof.gone());
        if at_origin {
            let mut revoked = c_config;
            enable_files(&mut revoked, b.hub(), c.hub(), false);
            c_node.configure(revoked).await;
        } else {
            let mut revoked = b_config;
            enable_files(&mut revoked, a.hub(), c.hub(), false);
            b_node.configure(revoked).await;
        }
        std::fs::write(path.join(C_FILES).join(READ_RELEASE_FILE), []).unwrap();
        assert!(
            tokio::time::timeout_at(deadline(), reading)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        wait_gone(proof).await;
        tokio::time::timeout_at(deadline(), async {
            loop {
                if matches!(
                    a_node
                        .client()
                        .await
                        .call(
                            c.hub(),
                            Command::Get {
                                address: target.address
                            },
                            deadline()
                        )
                        .await,
                    Ok(Reply::Terminal { .. })
                ) {
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
}
