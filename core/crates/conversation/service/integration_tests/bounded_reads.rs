use super::constants::{ESCAPED_PAGE_BYTE_LIMIT, EXCLUDING_PAGE_BYTE_LIMIT};
use super::fixtures::Fixture;
use serde_json::json;
use yolo_agent_contract::AgentKind;
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;

#[test]
fn metadata_only_is_not_a_completed_conversation_and_partial_content_is_explicit() {
    for kind in [AgentKind::Codex, AgentKind::ClaudeCode] {
        let fixture = Fixture::new(kind);
        let initial = fixture.page();
        assert_eq!(
            initial.availability,
            ConversationAvailability::NotYetWritten
        );
        initial.validate_for(&fixture.request).unwrap();
        fixture.append_bytes(b"malformed\n");
        let incomplete = fixture.page();
        assert_eq!(incomplete.availability, ConversationAvailability::Available);
        assert!(
            incomplete
                .partial
                .contains(&ConversationPartialReason::MalformedRecord)
        );
        fixture.message("recover after bad record");
        let recovered = fixture.page();
        assert_eq!(recovered.entries.len(), 1);
        assert_eq!(initial.history_generation, recovered.history_generation);
    }
}

#[test]
fn wire_byte_budget_is_enforced_with_escaped_text_and_entry_metadata() {
    for kind in [AgentKind::Codex, AgentKind::ClaudeCode] {
        let mut fixture = Fixture::new(kind);
        for _ in 0..4 {
            fixture.message(&"\"\\\n".repeat(8));
        }
        fixture.request.limits.bytes = ESCAPED_PAGE_BYTE_LIMIT;
        let page = fixture.page();
        assert!(!page.entries.is_empty());
        page.validate_for(&fixture.request).unwrap();
        assert!(page.next_cursor.is_some());
        fixture.request.limits.bytes = EXCLUDING_PAGE_BYTE_LIMIT;
        let omitted = fixture.page();
        assert!(omitted.entries.is_empty());
        assert_eq!(omitted.availability, ConversationAvailability::Available);
        assert!(
            omitted
                .partial
                .contains(&ConversationPartialReason::PageByteLimit)
        );
        omitted.validate_for(&fixture.request).unwrap();
    }
}

#[test]
fn unchanged_reply_requires_the_exact_generation_even_after_warm_cache() {
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.message("answer");
    let page = fixture.page();
    fixture.request.since_revision = Some(page.revision);
    fixture.request.since_generation = Some(Sha256Digest::repeated(9));
    let actual = fixture
        .reader
        .read(&fixture.binding, &fixture.request)
        .unwrap();
    assert!(matches!(actual, ConversationReadResult::Page { .. }));
    fixture.request.since_generation = Some(page.history_generation);
    let actual = fixture
        .reader
        .read(&fixture.binding, &fixture.request)
        .unwrap();
    assert!(matches!(actual, ConversationReadResult::Unchanged { .. }));
    actual.validate_for(&fixture.request).unwrap();
    fixture.request.binding.agent.execution_id = yolo_hub_identity::ExecutionId::new();
    let resumed = fixture
        .reader
        .read(&fixture.binding, &fixture.request)
        .unwrap();
    assert!(matches!(resumed, ConversationReadResult::Page { .. }));
}

#[test]
fn cleared_history_never_leaks_through_an_old_pagination_cursor() {
    let mut fixture = Fixture::new(AgentKind::ClaudeCode);
    fixture.message("before");
    let page = fixture.page();
    fixture.append(json!({"type":"system","subtype":"clear"}));
    fixture.request.cursor = Some(ConversationCursor {
        address: fixture.request.address.clone(),
        history_generation: page.history_generation,
        before_offset: 1,
    });
    assert_eq!(
        fixture.reader.read(&fixture.binding, &fixture.request),
        Err(ConversationError::HistoryChanged)
    );
    fixture.request.cursor = None;
    let cleared = fixture.page();
    assert!(cleared.entries.is_empty());
    assert_eq!(
        cleared.availability,
        ConversationAvailability::NotYetWritten
    );
    let length = std::fs::metadata(&fixture.path).unwrap().len();
    fixture.request.cursor = Some(ConversationCursor {
        address: fixture.request.address.clone(),
        history_generation: cleared.history_generation,
        before_offset: length + 1,
    });
    assert_eq!(
        fixture.reader.read(&fixture.binding, &fixture.request),
        Err(ConversationError::HistoryChanged)
    );
}
