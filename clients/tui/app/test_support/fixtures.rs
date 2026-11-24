#[path = "constants.rs"]
mod constants;
use constants::PROVIDER;
pub use constants::{
    ADDRESS_SCHEME, AGENT_WORKING_DIRECTORY, NEXT_AGENT_WORKING_DIRECTORY, QWEN_KIND,
    REMOVED_TERMINAL_RESOURCE,
};
use uuid::Uuid;
use yolo_hub_identity::*;
use yolo_hub_terminal_protocol::*;

pub const SESSION_NAME: &str = "Terminal";
pub const SESSION_DIRECTORY: &str = "/workspace";
pub const TTL_MS: u64 = 10_000;
pub const RENEW_MS: u64 = 5_000;
pub const EXPIRES_MS: i64 = 1_700_000_010_000;

pub fn session() -> TerminalSummary {
    let address = TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    };
    TerminalSummary::new(TerminalSummaryFields {
        presentation: Default::default(),
        address,
        name: SESSION_NAME.into(),
        cwd: SESSION_DIRECTORY.into(),
        archived: false,
        recovery: None,
        agent: None,
        runtime: Some(RuntimeSummary {
            target: RuntimeTarget {
                address,
                runtime_epoch: RuntimeEpoch::new(),
            },
            status: RuntimeStatus::Running,
            output_sequence: 0,
            size: Some(TerminalSize::default()),
            safe_termination: None,
        }),
        workspace: None,
    })
    .unwrap()
}

pub fn grant(session: &TerminalSummary) -> LeaseGrant {
    LeaseGrant::new(LeaseGrantFields {
        target: session.fields().runtime.unwrap().target,
        principal: WriterPrincipal {
            client_id: Uuid::new_v4(),
            controller_instance_id: Uuid::new_v4(),
        },
        generation: FIRST_LEASE_GENERATION,
        capability: SecretCapability::generate(),
        expires_at_ms: EXPIRES_MS,
        ttl_ms: TTL_MS,
        renew_after_ms: RENEW_MS,
    })
    .unwrap()
}

pub fn agent() -> (yolo_hub_agent_protocol::AgentSnapshot, TerminalSummary) {
    use yolo_hub_agent_protocol::*;
    let session = session();
    let target = session.fields().runtime.unwrap().target;
    let address = AgentAddress {
        hub_id: target.address.hub_id,
        agent_id: AgentId::new(),
    };
    let agent_target = AgentTarget {
        address,
        execution_id: ExecutionId::new(),
    };
    let mut fields = session.fields().clone();
    fields.agent = Some(agent_target);
    let session = TerminalSummary::new(fields).unwrap();
    let agent = AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(agent_target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE)
            .unwrap(),
        terminal: target,
        working_directory: None,
        provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: EXPIRES_MS as u64,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap();
    (agent, session)
}
