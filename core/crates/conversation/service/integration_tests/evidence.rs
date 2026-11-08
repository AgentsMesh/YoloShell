use super::fixtures::Fixture;
use std::fs;
use std::os::unix::fs::symlink;
use yolo_agent_contract::AgentKind;
use yolo_conversation_protocol::*;

#[test]
fn symlink_hardlink_and_foreign_evidence_are_not_read_as_bound_history() {
    for kind in [AgentKind::Codex, AgentKind::ClaudeCode] {
        let fixture = Fixture::new(kind);
        let outside = fixture.directory.path().join("outside");
        fs::rename(&fixture.path, &outside).unwrap();
        symlink(&outside, &fixture.path).unwrap();
        assert_eq!(
            fixture
                .reader
                .read(&fixture.binding, &fixture.request)
                .unwrap_err(),
            ConversationError::EvidenceUnavailable
        );
        fs::remove_file(&fixture.path).unwrap();
        fs::hard_link(&outside, &fixture.path).unwrap();
        assert_eq!(
            fixture
                .reader
                .read(&fixture.binding, &fixture.request)
                .unwrap_err(),
            ConversationError::EvidenceUnavailable
        );
    }
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.binding.provider_session_id = "different-native-session".into();
    assert_eq!(
        fixture
            .reader
            .resolve(&fixture.binding, fixture.request.binding)
            .unwrap_err(),
        ConversationError::BindingChanged
    );
}

#[test]
fn requests_are_scoped_to_both_origin_and_terminal_execution_association() {
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.request.address.hub_id = yolo_hub_identity::HubId::new();
    assert_eq!(
        fixture
            .reader
            .read(&fixture.binding, &fixture.request)
            .unwrap_err(),
        ConversationError::InvalidRequest
    );
    let mut fixture = Fixture::new(AgentKind::Codex);
    fixture.request.binding.runtime.address.terminal_id = yolo_hub_identity::TerminalId::new();
    assert_eq!(
        fixture
            .reader
            .read(&fixture.binding, &fixture.request)
            .unwrap_err(),
        ConversationError::BindingChanged
    );
    let mut fixture = Fixture::new(AgentKind::Codex);
    let mut address = fixture.request.address.clone();
    address.hub_id = yolo_hub_identity::HubId::new();
    fixture.request.cursor = Some(ConversationCursor {
        address,
        history_generation: yolo_crypto::Sha256Digest::repeated(1),
        before_offset: 1,
    });
    assert_eq!(
        fixture
            .reader
            .read(&fixture.binding, &fixture.request)
            .unwrap_err(),
        ConversationError::InvalidRequest
    );
}

#[test]
fn excessive_history_is_a_specific_bounded_failure() {
    let fixture = Fixture::new(AgentKind::Codex);
    fs::OpenOptions::new()
        .write(true)
        .open(&fixture.path)
        .unwrap()
        .set_len(yolo_conversation_transcript::MAX_HISTORY_SCAN_BYTES + 1)
        .unwrap();
    assert_eq!(
        fixture
            .reader
            .read(&fixture.binding, &fixture.request)
            .unwrap_err(),
        ConversationError::HistoryLimitExceeded
    );
}

#[test]
fn snapshot_fence_accepts_append_and_rejects_path_replacement() {
    let fixture = Fixture::new(AgentKind::Codex);
    let mut proof = yolo_codex::CodexAdapter
        .inspect_authoritative_evidence_with_hook(
            &fixture.path,
            std::path::Path::new(&fixture.binding.cwd),
            std::path::Path::new(&fixture.binding.data_home),
            || {},
        )
        .unwrap()
        .unwrap();
    assert!(
        proof
            .with_transcript_snapshot(|_, _| {
                fixture.message("racing append");
                Ok(())
            })
            .unwrap()
            .is_some()
    );
    let mut proof = yolo_codex::CodexAdapter
        .inspect_authoritative_evidence_with_hook(
            &fixture.path,
            std::path::Path::new(&fixture.binding.cwd),
            std::path::Path::new(&fixture.binding.data_home),
            || {},
        )
        .unwrap()
        .unwrap();
    assert!(
        proof
            .with_transcript_snapshot(|_, _| {
                fs::rename(&fixture.path, fixture.directory.path().join("retired")).unwrap();
                Ok(())
            })
            .unwrap()
            .is_none()
    );
}
