use super::constants::{CONTEXT_CLEARED, EVENT_MESSAGE, MULTI_ENTRY_PAGE, SINGLE_ENTRY_PAGE};
use super::fixtures::Fixture;
use serde_json::json;
use std::fs;
use yolo_agent_contract::AgentKind;
use yolo_conversation_protocol::*;

#[test]
fn providers_page_append_and_resume_preserve_conversation_without_changing_hook_state() {
    for kind in [AgentKind::Codex, AgentKind::ClaudeCode] {
        let mut fixture = Fixture::new(kind);
        let binding = fixture.binding.clone();
        for text in ["one", "two", "three"] {
            fixture.message(text);
        }
        fixture.request.limits.entries = MULTI_ENTRY_PAGE;
        let latest = fixture.page();
        assert_eq!(latest.entries.len(), 2);
        let old_cursor = latest.next_cursor.clone().unwrap();
        fixture.message("four");
        let appended = fixture.page();
        assert_eq!(latest.history_generation, appended.history_generation);
        assert_ne!(latest.revision, appended.revision);
        assert_eq!(latest.entries[1].id, appended.entries[0].id);
        fixture.request.cursor = Some(old_cursor);
        let older = fixture.page();
        assert_eq!(older.entries.len(), 1);
        assert!(older.next_cursor.is_none());
        fixture.request.since_revision = Some(older.revision);
        fixture.request.since_generation = Some(older.history_generation);
        assert!(matches!(
            fixture
                .reader
                .read(&fixture.binding, &fixture.request)
                .unwrap(),
            ConversationReadResult::Unchanged { .. }
        ));
        let mut resumed = fixture.request.binding;
        resumed.agent.execution_id = yolo_hub_identity::ExecutionId::new();
        resumed.runtime.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
        assert_eq!(
            fixture
                .reader
                .resolve(&fixture.binding, resumed)
                .unwrap()
                .address,
            fixture.request.address
        );
        assert_eq!(fixture.binding, binding);
    }
}

#[test]
fn same_inode_rewrite_plus_append_changes_generation_and_rejects_old_cursors() {
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.request.limits.entries = SINGLE_ENTRY_PAGE;
    fixture.message("one");
    fixture.message("two");
    let old = fixture.page();
    fixture.request.cursor = old.next_cursor;
    let bytes = fs::read_to_string(&fixture.path)
        .unwrap()
        .replace("one", "new");
    fs::write(&fixture.path, bytes).unwrap();
    fixture.message("three");
    assert_eq!(
        fixture
            .reader
            .read(&fixture.binding, &fixture.request)
            .unwrap_err(),
        ConversationError::HistoryChanged
    );
    fixture.request.cursor = None;
    assert_ne!(old.history_generation, fixture.page().history_generation);
}

#[test]
fn truncate_replace_and_clear_invalidate_history_without_rebinding_to_another_session() {
    for replacement in [false, true] {
        let mut fixture = Fixture::new(AgentKind::Codex);
        fixture.request.limits.entries = SINGLE_ENTRY_PAGE;
        fixture.message("one");
        fixture.message("two");
        let old = fixture.page();
        fixture.request.cursor = old.next_cursor;
        let bytes = fs::read_to_string(&fixture.path).unwrap();
        let header = format!("{}\n", bytes.lines().next().unwrap());
        if replacement {
            fs::rename(&fixture.path, fixture.directory.path().join("old-file")).unwrap();
        }
        fs::write(&fixture.path, header).unwrap();
        assert_eq!(
            fixture
                .reader
                .read(&fixture.binding, &fixture.request)
                .unwrap_err(),
            ConversationError::HistoryChanged
        );
    }
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.message("old private text");
    let old = fixture.page();
    fixture.append(json!({"type":EVENT_MESSAGE,"payload":{"type":CONTEXT_CLEARED}}));
    fixture.message("new text");
    let new = fixture.page();
    assert_ne!(old.history_generation, new.history_generation);
    assert_eq!(new.entries.len(), 1);
    fixture.request.cursor = Some(ConversationCursor {
        address: fixture.request.address.clone(),
        history_generation: old.history_generation,
        before_offset: 1,
    });
    assert_eq!(
        fixture
            .reader
            .read(&fixture.binding, &fixture.request)
            .unwrap_err(),
        ConversationError::HistoryChanged
    );
}

#[test]
fn torn_tail_is_not_published_until_committed_as_a_complete_record() {
    let fixture = Fixture::new(AgentKind::ClaudeCode);
    fixture.append_bytes(b"{\"type\":\"assistant\",\"message\":{\"content\":\"pending\"}}");
    let incomplete = fixture.page();
    assert!(incomplete.entries.is_empty());
    assert!(
        incomplete
            .partial
            .contains(&ConversationPartialReason::IncompleteRecord)
    );
    fixture.append_bytes(b"\n");
    let complete = fixture.page();
    assert_eq!(complete.entries.len(), 1);
    assert!(
        !complete
            .partial
            .contains(&ConversationPartialReason::IncompleteRecord)
    );
    assert_eq!(incomplete.history_generation, complete.history_generation);
}
