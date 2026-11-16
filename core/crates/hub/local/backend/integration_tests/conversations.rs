use super::{cleanup_rpc::*, conversation_support::*, native_support::*};
use yolo_agent_contract::AgentState;
use yolo_conversation_protocol::*;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn public_reads_are_worker_bound_and_activity_changes_do_not_invalidate_content() {
    let fixture = ProtocolFixture::new().with_agents();
    let evidence = Evidence::new();
    evidence.message("native answer");
    let before = &evidence.view;
    let observed = observation(before, SOURCE_REVISION);
    let snapshot = observe(&fixture, before, Some(&observed)).await;
    let target = binding(&snapshot);
    let command = Command::ResolveConversation { binding: target };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture
                .backend
                .execute(&command),
            two_fences(&fixture, before, before)
        )
    })
    .await;
    let Reply::Conversation { conversation } = reply.unwrap() else {
        panic!("expected descriptor")
    };
    conversation.validate_for(target).unwrap();
    let mut after = before.clone();
    after.agent.as_mut().unwrap().state = AgentState::WaitingForPermission;
    after.agent.as_mut().unwrap().updated_at_ms += 1;
    let request = request(&conversation);
    let command = Command::ReadConversation {
        request: request.clone(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&command),
            two_fences(&fixture, before, &after)
        )
    })
    .await;
    let Reply::ConversationRead { reading } = reply.unwrap() else {
        panic!("expected content")
    };
    reading.validate_for(&request).unwrap();
    let ConversationReadResult::Page { page } = reading else {
        panic!()
    };
    assert_eq!(page.entries.len(), 1);
    assert!(
        matches!(&page.entries[0].parts[0], ConversationPart::Text { text } if text == "native answer")
    );
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn content_is_discarded_if_native_process_changes_during_a_read() {
    let fixture = ProtocolFixture::new().with_agents();
    let evidence = Evidence::new();
    evidence.message("must not escape stale execution");
    let before = &evidence.view;
    let observed = observation(before, SOURCE_REVISION);
    let snapshot = observe(&fixture, before, Some(&observed)).await;
    let mut after = before.clone();
    after.agent.as_mut().unwrap().process_start_time =
        Some(observed.binding().process_start_time.unwrap() + 1);
    after.agent.as_mut().unwrap().updated_at_ms += 1;
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.resolve_conversation(binding(&snapshot)),
            two_fences(&fixture, before, &after)
        )
    })
    .await;
    assert_eq!(
        reply.unwrap(),
        Reply::ConversationUnavailable {
            error: ConversationError::BindingChanged
        }
    );
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn conditional_watch_returns_unchanged_then_wakes_for_appended_native_content() {
    let fixture = ProtocolFixture::new().with_agents();
    let evidence = Evidence::new();
    evidence.message("first");
    let view = &evidence.view;
    let observed = observation(view, SOURCE_REVISION);
    let snapshot = observe(&fixture, view, Some(&observed)).await;
    let descriptor = fixture
        .backend
        .conversations
        .resolve(view.agent.as_ref().unwrap(), binding(&snapshot))
        .unwrap();
    let mut request = request(&descriptor);
    let page = match fixture
        .backend
        .conversations
        .read(view.agent.as_ref().unwrap(), &request)
        .unwrap()
    {
        ConversationReadResult::Page { page } => page,
        _ => panic!(),
    };
    request.since_revision = Some(page.revision);
    request.since_generation = Some(page.history_generation);
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.watch_conversation(&request, 0),
            two_fences(&fixture, view, view)
        )
    })
    .await;
    assert!(matches!(
        reply.unwrap(),
        Reply::ConversationRead {
            reading: ConversationReadResult::Unchanged { .. }
        }
    ));
    let peer = async {
        two_fences(&fixture, view, view).await;
        evidence.message("second");
        two_fences(&fixture, view, view).await;
    };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture
                .backend
                .watch_conversation(&request, MAX_WATCH_WAIT_MS),
            peer
        )
    })
    .await;
    let Reply::ConversationRead {
        reading: ConversationReadResult::Page { page },
    } = reply.unwrap()
    else {
        panic!()
    };
    assert_eq!(page.entries.len(), 2);
    page.validate_for(&request).unwrap();
    fixture.assert_no_pending_connection();
}
