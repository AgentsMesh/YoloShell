use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_identity::*;

pub fn descriptor(hub_id: HubId) -> ConversationDescriptor {
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
        provider_session_id: "session".into(),
        source: ConversationSource::NativeTranscript,
    }
}

pub fn request(descriptor: &ConversationDescriptor) -> ConversationReadRequest {
    ConversationReadRequest {
        address: descriptor.address.clone(),
        binding: descriptor.binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_generation: None,
        since_revision: None,
    }
}

pub fn reading(descriptor: &ConversationDescriptor) -> ConversationReadResult {
    ConversationReadResult::Page {
        page: ConversationPage {
            conversation: descriptor.clone(),
            history_generation: Sha256Digest::repeated(2),
            revision: Sha256Digest::repeated(3),
            availability: ConversationAvailability::NotYetWritten,
            entries: vec![],
            partial: vec![],
            next_cursor: None,
        },
    }
}

pub fn agent(descriptor: &ConversationDescriptor) -> yolo_hub_agent_protocol::AgentSnapshot {
    use yolo_hub_agent_protocol::*;
    AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(
            descriptor.binding.agent,
            uuid::Uuid::new_v4(),
            FIRST_OBSERVATION_SEQUENCE,
        )
        .unwrap(),
        terminal: descriptor.binding.runtime,
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
