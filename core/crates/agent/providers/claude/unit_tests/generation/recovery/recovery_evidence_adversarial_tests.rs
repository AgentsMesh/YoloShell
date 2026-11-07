use super::recovery_event_support::ALL_EVENTS_COUNT;
use super::recovery_test_support::RecoveryFixture;
use std::io::Write;

#[test]
fn recovery_ignores_transcript_state_records_after_queued_hooks() {
    let fixture = RecoveryFixture::new(ALL_EVENTS_COUNT);
    let _guards = fixture.gone_guards();
    append_non_authoritative_state_record(&fixture);

    let binding = fixture.load().unwrap().unwrap();
    assert_eq!(binding.state, crate::agent::AgentState::WaitingForInput);
    assert!(
        crate::claude_hooks::outbox::pending_recovery(&fixture.outbox, fixture.credentials)
            .unwrap()
            .is_none()
    );
    assert!(fixture.binding_store.path().exists());
}

fn append_non_authoritative_state_record(fixture: &RecoveryFixture) {
    let transcript = fixture
        .base
        .transcript(&fixture.base.begin.provider_session_id);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(transcript)
        .unwrap();
    let record = serde_json::json!({
        (crate::agent::claude::constants::RECORD_TYPE_KEY):
            crate::agent::claude::constants::USER_RECORD,
    });
    serde_json::to_writer(&mut file, &record).unwrap();
    write!(file, "{}", crate::agent::constants::COMMAND_LINE_ENDING).unwrap();
    file.sync_data().unwrap();
}
