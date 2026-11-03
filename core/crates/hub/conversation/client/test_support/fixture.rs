use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_identity::*;

pub fn conversation() -> ConversationDescriptor {
    let hub_id = HubId::new();
    ConversationDescriptor {
        address: ConversationAddress {
            hub_id,
            conversation_id: Sha256Digest::repeated(1),
        },
        binding: ConversationBinding {
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
        },
        provider: ConversationProvider::codex(),
        provider_session_id: "native-session".into(),
        source: ConversationSource::NativeTranscript,
    }
}

pub fn request(conversation: &ConversationDescriptor) -> ConversationReadRequest {
    ConversationReadRequest {
        address: conversation.address.clone(),
        binding: conversation.binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: None,
        since_generation: None,
    }
}

pub fn page(conversation: ConversationDescriptor) -> ConversationPage {
    ConversationPage {
        conversation,
        history_generation: Sha256Digest::repeated(2),
        revision: Sha256Digest::repeated(3),
        availability: ConversationAvailability::Available,
        partial: Vec::new(),
        next_cursor: None,
        entries: vec![ConversationEntry {
            id: Sha256Digest::repeated(4),
            native_id: None,
            parent_id: None,
            timestamp: None,
            role: ConversationRole::Assistant,
            parts: vec![ConversationPart::Text {
                text: "answer".into(),
            }],
        }],
    }
}

pub fn agent(conversation: &ConversationDescriptor) -> yolo_hub_agent_protocol::AgentSnapshot {
    use yolo_hub_agent_protocol::*;
    AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(
            conversation.binding.agent,
            uuid::Uuid::new_v4(),
            FIRST_OBSERVATION_SEQUENCE,
        )
        .unwrap(),
        terminal: conversation.binding.runtime,
        provider: ObservedProvider::new("codex".into(), None, None).unwrap(),
        working_directory: None,
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: 1,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap()
}
