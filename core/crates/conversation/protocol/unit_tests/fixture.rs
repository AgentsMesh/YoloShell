use super::*;

pub(super) fn fixture() -> (ConversationReadRequest, ConversationPage) {
    let hub_id = HubId::new();
    let binding = ConversationBinding {
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
    };
    let address = ConversationAddress {
        hub_id,
        conversation_id: Sha256Digest::repeated(1),
    };
    let request = ConversationReadRequest {
        address: address.clone(),
        binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: None,
        since_generation: None,
    };
    let page = ConversationPage {
        conversation: ConversationDescriptor {
            address,
            binding,
            provider: ConversationProvider::codex(),
            provider_session_id: "session".into(),
            source: ConversationSource::NativeTranscript,
        },
        history_generation: Sha256Digest::repeated(2),
        revision: Sha256Digest::repeated(3),
        availability: ConversationAvailability::Available,
        partial: vec![],
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
    };
    (request, page)
}
