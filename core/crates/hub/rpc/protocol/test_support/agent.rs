use super::{constants::*, *};
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId};

pub fn agent() -> AgentSnapshot {
    let target = target();
    let cursor = ObservationCursor::new(
        AgentTarget {
            address: AgentAddress {
                hub_id: target.address.hub_id,
                agent_id: AgentId::from_uuid(id(AGENT_ID)).unwrap(),
            },
            execution_id: ExecutionId::from_uuid(id(EXECUTION_ID)).unwrap(),
        },
        id(AGENT_STREAM_ID),
        FIRST_OBSERVATION_SEQUENCE,
    )
    .unwrap();
    AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor,
        terminal: target,
        provider: ObservedProvider::new(PROVIDER_KIND.into(), None, None).unwrap(),
        working_directory: None,
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: FIRST_OBSERVATION_SEQUENCE,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap()
}
