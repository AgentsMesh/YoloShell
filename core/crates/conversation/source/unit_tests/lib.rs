use super::*;
use yolo_agent_contract::{AgentBinding, AgentEvidence, AgentKind, AgentState};
use yolo_conversation_protocol::{ConversationError, ConversationProvider};
use yolo_conversation_transcript::{ParsedRecord, TranscriptParser};

struct FixtureParser;

impl TranscriptParser for FixtureParser {
    fn parse(&self, _: &serde_json::Value) -> ParsedRecord {
        ParsedRecord::default()
    }
}

struct FixtureSource;

impl ConversationSource for FixtureSource {
    fn kind(&self) -> AgentKind {
        AgentKind::Other("fixture".into())
    }

    fn conversation_token(&self) -> ConversationProvider {
        ConversationProvider::new("fixture").unwrap()
    }

    fn parser(&self) -> &'static dyn TranscriptParser {
        &FixtureParser
    }

    fn verify(&self, _: &AgentBinding) -> Result<(), ConversationError> {
        Err(ConversationError::EvidenceUnavailable)
    }

    fn with_evidence(
        &self,
        _: &AgentBinding,
        _: &mut dyn FnMut(&EvidenceFile<'_>) -> Result<ConversationReadOutcome, ConversationError>,
    ) -> Result<ConversationReadOutcome, ConversationError> {
        Err(ConversationError::EvidenceUnavailable)
    }
}

#[test]
fn sources_lookup_by_kind_and_token_and_unknowns_fail_closed() {
    let source: std::sync::Arc<dyn ConversationSource> = std::sync::Arc::new(FixtureSource);
    let sources = ConversationSources::new([source]);
    let kind = AgentKind::Other("fixture".into());
    let token = ConversationProvider::new("fixture").unwrap();
    assert!(sources.source_for_kind(&kind).is_some());
    assert!(sources.source_for_token(&token).is_some());
    assert!(sources.source_for_kind(&AgentKind::Codex).is_none());
    assert!(
        sources
            .source_for_token(&ConversationProvider::codex())
            .is_none()
    );
    assert!(ConversationSources::default().is_empty());
}

#[test]
fn evidence_binding_requires_the_exact_native_identity_and_private_paths() {
    let binding = AgentBinding {
        schema_version: 5,
        terminal_id: uuid::Uuid::new_v4(),
        kind: AgentKind::Codex,
        provider_session_id: "session".into(),
        executable: "codex".into(),
        cwd: "/project".into(),
        evidence_path: "/store/session".into(),
        data_home: "/store".into(),
        process_pid: None,
        process_start_time: None,
        state: AgentState::Unknown,
        recovery_availability: yolo_agent_contract::AgentRecoveryAvailability::Unavailable,
        detected_at_ms: 0,
        updated_at_ms: 0,
    };
    let evidence = AgentEvidence {
        provider_session_id: "session".into(),
        cwd: "/project".into(),
        path: "/store/session".into(),
        data_home: "/store".into(),
        state: AgentState::Unknown,
    };
    assert!(require_binding(&binding, &evidence).is_ok());
    let mut changed = evidence;
    changed.provider_session_id.push_str("-other");
    assert_eq!(
        require_binding(&binding, &changed),
        Err(ConversationError::BindingChanged)
    );
}
