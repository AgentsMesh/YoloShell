use super::constants::*;
use uuid::Uuid;
use yolo_conversation_protocol::ConversationBinding;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::*;

pub fn agent() -> AgentSnapshot {
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: HubId::new(),
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap(),
        terminal: RuntimeTarget {
            address: TerminalAddress {
                hub_id: target.address.hub_id,
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
        provider: ObservedProvider::new(PROVIDER.into(), None, Some(NATIVE_SESSION.into()))
            .unwrap(),
        working_directory: None,
        lifecycle: Lifecycle::Running {},
        activity: Activity::WaitingForInput,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: 1,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities {
            interrupt: Capability::Available {},
            submit_prompt: Capability::Available {},
            respond_interaction: Capability::Unsupported {},
        },
        interaction: None,
    })
    .unwrap()
}
pub fn binding(agent: &AgentSnapshot) -> ConversationBinding {
    ConversationBinding {
        agent: agent.cursor().target(),
        runtime: agent.fields().terminal,
    }
}
