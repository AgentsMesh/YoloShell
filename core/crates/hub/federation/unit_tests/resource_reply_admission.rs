use super::{fixture::NodeFixture, resource_support::*};
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::required_rights;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn native_origin_rechecks_valid_read_calls_before_egress_after_expiry_or_retirement() {
    for index in 0..resource_reads(HubId::new()).len() {
        let fixture = super::origin_support::OriginFixture::new(index).await;
        let command = resource_reads(fixture.node.hub_id()).remove(index);
        let valid = fixture.call(command.clone(), false);
        fixture
            .node
            .admit_read_reply(fixture.peer.identity, &valid)
            .await
            .unwrap();
        let expired = fixture.call(command, true);
        assert_eq!(
            fixture
                .node
                .admit_read_reply(fixture.peer.identity, &expired)
                .await,
            Err(PublicError::Unauthorized)
        );
        fixture.node.begin_shutdown();
        assert_eq!(
            fixture
                .node
                .admit_read_reply(fixture.peer.identity, &valid)
                .await,
            Err(PublicError::Unreachable)
        );
    }
}

#[tokio::test]
async fn conversation_and_layout_replies_recheck_expiry_and_signed_policy_revocation() {
    let allowed = resource_reads(HubId::new());
    for index in 0..allowed.len() {
        let fixture = NodeFixture::with_resource_access(&allowed).await;
        let command = resource_reads(fixture.origin.identity.hub()).remove(index);
        let valid = call(&fixture, command.clone(), false).await;
        fixture.node.admit_entry_read_reply(&valid).await.unwrap();
        let expired = call(&fixture, command.clone(), true).await;
        assert_eq!(
            fixture.node.admit_entry_read_reply(&expired).await,
            Err(PublicError::Unauthorized)
        );
        revoke_resource_rights(&fixture).await;
        let now = crate::node::now_ms().unwrap();
        assert!(
            fixture
                .node
                .book
                .lock()
                .await
                .route_for(
                    fixture.origin.identity.hub(),
                    &required_rights(&command).expect("resource read rights"),
                    now,
                )
                .unwrap()
                .is_none()
        );
        assert_eq!(
            fixture.node.admit_entry_read_reply(&valid).await,
            Err(PublicError::Unreachable)
        );
    }
}

#[tokio::test]
async fn file_permission_never_substitutes_for_conversation_or_layout_read_permission() {
    let fixture =
        NodeFixture::with_resource_access(&[Command::ListFileRoots { after: None }]).await;
    for command in resource_reads(fixture.origin.identity.hub()) {
        let request = call(&fixture, command, false).await;
        assert_eq!(
            fixture.node.admit_entry_read_reply(&request).await,
            Err(PublicError::Unreachable)
        );
        assert_eq!(
            fixture
                .node
                .admit_read_reply(fixture.relay.identity, &request)
                .await,
            Err(PublicError::Unauthorized)
        );
    }
}
