use super::fixture::NodeFixture;
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_delegation::{DelegationChain, SignedGrant};
use yolo_hub_identity::*;
use yolo_hub_peer_protocol::*;
use yolo_hub_peer_protocol_test_support::support::{child_claims, observe, root_claims};
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_presentation_protocol::LayoutAddress;

const CONVERSATION_SEED: &str = "read-admission-conversation";
const NEXT_REVISION: u64 = 1;

pub fn resource_reads(hub_id: HubId) -> Vec<Command> {
    let binding = ConversationBinding {
        agent: AgentTarget {
            address: AgentAddress {
                hub_id,
                agent_id: AgentId::new(),
            },
            execution_id: ExecutionId::new(),
        },
        runtime: RuntimeTarget {
            address: TerminalAddress {
                hub_id,
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
    };
    let request = ConversationReadRequest {
        address: ConversationAddress {
            hub_id,
            conversation_id: Sha256Digest::capture(&CONVERSATION_SEED).unwrap(),
        },
        binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_generation: None,
        since_revision: None,
    };
    vec![
        Command::ResolveConversation { binding },
        Command::ReadConversation {
            request: request.clone(),
        },
        Command::WatchConversation {
            request,
            wait_ms: MAX_WATCH_WAIT_MS,
        },
        Command::ListLayouts {},
        Command::GetLayout {
            address: LayoutAddress {
                hub_id,
                layout_id: uuid::Uuid::new_v4(),
            },
        },
    ]
}

pub async fn call(fixture: &NodeFixture, command: Command, expired: bool) -> Signed<Call> {
    let now = crate::node::now_ms().unwrap();
    let origin = fixture.origin.identity.hub();
    let authority = fixture
        .node
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
            request: Request::new(origin, command).unwrap(),
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
        fixture.node.state.as_ref(),
    )
    .unwrap()
}

/// Install a newly signed origin policy epoch retaining observation only.
pub async fn revoke_resource_rights(fixture: &NodeFixture) {
    let now = crate::node::now_ms().unwrap();
    let mut book = fixture.node.book.lock().await;
    let mut catalogue = book
        .latest_for(fixture.origin.identity.hub())
        .unwrap()
        .payload
        .clone();
    catalogue.revision += NEXT_REVISION;
    catalogue.policy_epoch += NEXT_REVISION;
    catalogue.issued_at_ms = now;
    catalogue.expires_at_ms = now + MAX_CATALOGUE_LIFETIME_MS;
    let mut claims = root_claims(&fixture.origin, &fixture.relay);
    claims.rights = observe();
    claims.policy_epoch = catalogue.policy_epoch;
    claims.not_before_ms = now;
    claims.expires_at_ms = catalogue.expires_at_ms;
    let root = SignedGrant::issue(claims, &fixture.origin).unwrap();
    let mut claims = child_claims(&root, &fixture.relay);
    claims.recipient = fixture.node.state.peer_identity();
    let child = SignedGrant::issue(claims, &fixture.relay).unwrap();
    let revision = catalogue.revision;
    let export = ExportedOrigin {
        catalogue: Signed::new(catalogue, Purpose::Catalogue, &fixture.origin).unwrap(),
        authority: DelegationChain::new(vec![root, child]).unwrap(),
        reachable: true,
    };
    book.install(
        fixture.relay.identity,
        fixture.link,
        revision,
        vec![export],
        now,
    )
    .unwrap();
}
