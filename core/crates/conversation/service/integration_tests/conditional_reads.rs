use super::fixtures::Fixture;
use std::fs;
use yolo_agent_contract::AgentKind;
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;

#[test]
fn a_different_conversation_at_the_same_hub_cannot_reuse_an_agent_binding() {
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.request.address.conversation_id = Sha256Digest::repeated(9);
    assert_eq!(
        fixture.reader.read(&fixture.binding, &fixture.request),
        Err(ConversationError::BindingChanged)
    );
}

#[test]
fn unchanged_content_after_cache_miss_keeps_its_revision_but_not_a_forged_generation() {
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.message("stable content");
    let initial = fixture.page();
    fixture.request.since_revision = Some(initial.revision);
    fixture.request.since_generation = Some(initial.history_generation);
    // A same-byte rewrite changes the ctime stamp and forces the complete prefix scan.
    let bytes = fs::read(&fixture.path).unwrap();
    fs::write(&fixture.path, &bytes).unwrap();
    fs::File::open(&fixture.path)
        .unwrap()
        .set_modified(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    assert!(
        matches!(fixture.reader.read(&fixture.binding, &fixture.request).unwrap(),
        ConversationReadResult::Unchanged { history_generation, revision }
            if history_generation == initial.history_generation && revision == initial.revision)
    );
    fixture.request.since_generation = Some(Sha256Digest::repeated(9));
    fs::write(&fixture.path, &bytes).unwrap();
    fs::File::open(&fixture.path)
        .unwrap()
        .set_modified(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    let page = fixture.page();
    assert_eq!(page.revision, initial.revision);
    assert_eq!(page.history_generation, initial.history_generation);
}

#[test]
fn a_cursor_with_current_generation_cannot_read_below_the_clear_floor() {
    let mut fixture = Fixture::new(AgentKind::ClaudeCode);
    fixture.message("retired history");
    fixture.append(serde_json::json!({"type":"system","subtype":"clear"}));
    let current = fixture.page();
    fixture.request.cursor = Some(ConversationCursor {
        address: fixture.request.address.clone(),
        history_generation: current.history_generation,
        before_offset: 0,
    });
    assert_eq!(
        fixture.reader.read(&fixture.binding, &fixture.request),
        Err(ConversationError::HistoryChanged)
    );
}
