use crate::wait::{Progress, progress};
use crate::wait_pages::new_native_entries;
use yolo_conversation_protocol::{ConversationEntry, ConversationPart, ConversationRole};
use yolo_crypto::Sha256Digest;
use yolo_hub_agent_protocol::Activity;

#[test]
fn a_settled_working_turn_gets_a_transcript_grace_period() {
    assert_eq!(
        progress(Activity::Working, true, false, false),
        Progress::Running
    );
    assert_eq!(
        progress(Activity::WaitingForInput, true, false, false),
        Progress::AwaitingReply
    );
    assert_eq!(
        progress(Activity::WaitingForInput, true, false, true),
        Progress::Completed
    );
}

#[test]
fn an_observed_assistant_entry_completes_without_working_history() {
    assert_eq!(
        progress(Activity::WaitingForInput, false, true, false),
        Progress::Completed
    );
    assert_eq!(
        progress(Activity::WaitingForInput, false, false, false),
        Progress::Running
    );
    assert_eq!(
        progress(Activity::WaitingForApproval, true, true, false),
        Progress::Running
    );
}

fn entry(id: u8, native: Option<&str>) -> ConversationEntry {
    ConversationEntry {
        id: Sha256Digest::repeated(id),
        parent_id: None,
        native_id: native.map(str::to_owned),
        timestamp: None,
        role: ConversationRole::Assistant,
        parts: vec![ConversationPart::Text { text: "x".into() }],
    }
}

#[test]
fn a_rewritten_history_keeps_only_new_native_records() {
    let known = vec!["old".to_owned()];
    let entries = vec![entry(1, Some("old")), entry(2, Some("new")), entry(3, None)];
    let new = new_native_entries(&known, &entries);
    assert_eq!(new.len(), 1);
    assert_eq!(new[0].native_id.as_deref(), Some("new"));
}
