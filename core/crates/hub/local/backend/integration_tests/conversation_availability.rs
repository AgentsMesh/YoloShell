use super::{cleanup_rpc::*, conversation_support::*, connectors::get, native_support::*};
use yolo_conversation_protocol::*;
use yolo_hub_rpc_protocol::Reply;
use yolo_runtime::protocol::{ErrorBody, ResponseEnvelope, error_code};

#[tokio::test]
async fn missing_live_witness_before_or_after_read_is_unavailable_without_false_rebinding() {
    for before_read in [false, true] {
        let fixture = ProtocolFixture::new().with_agents();
        let evidence = Evidence::new();
        evidence.message("retained conversation");
        let view = &evidence.view;
        let observed = observation(view, SOURCE_REVISION);
        let snapshot = observe(&fixture, view, Some(&observed)).await;
        let target = binding(&snapshot);
        let descriptor = fixture.backend.conversations.resolve(view.agent.as_ref().unwrap(), target).unwrap();
        let request = request(&descriptor);
        let peer = async {
            if !before_read { fence(&fixture, view).await; }
            fixture.reply(|request| get(request, view)).await;
            fixture.reply(|request| native(request, view, None)).await;
        };
        let (reply, ()) = bounded(async {
            tokio::join!(fixture.backend.read_conversation(&request), peer)
        }).await;
        assert_eq!(reply.unwrap(), Reply::ConversationUnavailable { error: ConversationError::EvidenceUnavailable });
        // The same execution can recover without manufacturing another binding.
        let (recovered, ()) = bounded(async {
            tokio::join!(fixture.backend.read_conversation(&request), two_fences(&fixture, view, view))
        }).await;
        assert!(matches!(recovered.unwrap(), Reply::ConversationRead { .. }));
        fixture.assert_no_pending_connection();
    }
}

#[tokio::test]
async fn failed_post_read_host_query_preserves_unavailability_and_discards_unverified_content() {
    let fixture = ProtocolFixture::new().with_agents();
    let evidence = Evidence::new();
    let view = &evidence.view;
    let observed = observation(view, SOURCE_REVISION);
    let snapshot = observe(&fixture, view, Some(&observed)).await;
    let peer = async {
        fence(&fixture, view).await;
        fixture.reply(|request| ResponseEnvelope::failure(request,
            ErrorBody::new(error_code::WORKER_UNREACHABLE, error_code::WORKER_UNREACHABLE, true))).await;
    };
    let (reply, ()) = bounded(async {
        tokio::join!(fixture.backend.resolve_conversation(binding(&snapshot)), peer)
    }).await;
    assert_eq!(reply.unwrap(), Reply::ConversationUnavailable { error: ConversationError::EvidenceUnavailable });
    fixture.assert_no_pending_connection();
}
