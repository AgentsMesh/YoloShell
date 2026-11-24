#[path = "../test_support/node_inventory.rs"]
mod fixture;
use fixture::NodeFixture;
use yolo_hub_peer_protocol::{Call, MAX_CALL_LIFETIME_MS, Purpose, Signed};
use yolo_hub_peer_protocol_test_support::support::observe;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::PublicError;

#[path = "../test_support/read_reply_admission.rs"]
mod resource_support;
#[path = "../test_support/origin_read_reply.rs"]
mod origin_support;
#[path = "resource_reply_admission.rs"]
mod resources;

async fn call(fixture: &NodeFixture, files: bool, expired: bool) -> Signed<Call> {
    let node = &fixture.node;
    let now = crate::node::now_ms().unwrap();
    let origin = fixture.origin.identity.hub();
    let authority = node
        .book
        .lock()
        .await
        .route_for(origin, &observe(), now)
        .unwrap()
        .unwrap()
        .authority
        .clone();
    Signed::new(
        Call {
            request: Request::new(
                origin,
                if files {
                    Command::ListFileRoots { after: None }
                } else {
                    Command::ListHubs { after: None }
                },
            )
            .unwrap(),
            authority,
            issued_at_ms: if expired {
                now.saturating_sub(MAX_CALL_LIFETIME_MS)
            } else {
                now
            },
            expires_at_ms: if expired {
                now
            } else {
                now + MAX_CALL_LIFETIME_MS
            },
        },
        Purpose::Call,
        node.state.as_ref(),
    )
    .unwrap()
}

#[tokio::test]
async fn file_reply_requires_an_unexpired_call_and_a_current_authorized_route() {
    let fixture =
        NodeFixture::with_resource_access(&[Command::ListFileRoots { after: None }]).await;
    let valid = call(&fixture, true, false).await;
    fixture.node.admit_entry_read_reply(&valid).await.unwrap();
    assert_eq!(
        fixture
            .node
            .admit_entry_read_reply(&call(&fixture, true, true).await)
            .await,
        Err(PublicError::Unauthorized)
    );
    fixture
        .node
        .book
        .lock()
        .await
        .disconnect(fixture.relay.identity.hub(), fixture.link)
        .unwrap();
    assert_eq!(
        fixture.node.admit_entry_read_reply(&valid).await,
        Err(PublicError::Unreachable)
    );
}

#[tokio::test]
async fn observation_rights_cannot_authorize_file_bytes_and_shutdown_is_rechecked() {
    let fixture = NodeFixture::new().await;
    let request = call(&fixture, true, false).await;
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
    fixture.node.begin_shutdown();
    assert_eq!(
        fixture.node.admit_entry_read_reply(&request).await,
        Err(PublicError::Unreachable)
    );
    assert_eq!(
        fixture
            .node
            .admit_read_reply(fixture.relay.identity, &request)
            .await,
        Err(PublicError::Unreachable)
    );
}

#[tokio::test]
async fn read_egress_fences_do_not_reclassify_other_operation_results() {
    let fixture = NodeFixture::new().await;
    let request = call(&fixture, false, false).await;
    fixture.node.admit_entry_read_reply(&request).await.unwrap();
    fixture
        .node
        .admit_read_reply(fixture.relay.identity, &request)
        .await
        .unwrap();
}
