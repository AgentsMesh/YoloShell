use yolo_hub_agent_protocol::*;
use yolo_hub_identity::*;
use yolo_hub_terminal_protocol::*;

const TERMINAL_NAME: &str = "terminal";
const DIRECTORY: &str = "/workspace";
const PROVIDER: &str = yolo_hub_agent_protocol::PROVIDER_CODEX;

pub fn terminal() -> TerminalSummaryFields {
    let address = TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    };
    TerminalSummaryFields {
        presentation: Default::default(),
        address,
        name: TERMINAL_NAME.into(),
        cwd: DIRECTORY.into(),
        archived: false,
        agent: None,
        recovery: None,
        runtime: Some(RuntimeSummary {
            target: RuntimeTarget {
                address,
                runtime_epoch: RuntimeEpoch::new(),
            },
            status: RuntimeStatus::Running,
            output_sequence: u64::default(),
            size: None,
            safe_termination: None,
        }),
        workspace: None,
    }
}

pub fn agent(terminal: &mut TerminalSummaryFields) -> AgentSnapshot {
    let address = AgentAddress {
        hub_id: terminal.address.hub_id,
        agent_id: AgentId::new(),
    };
    let target = AgentTarget {
        address,
        execution_id: ExecutionId::new(),
    };
    terminal.agent = Some(target);
    AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(
            target,
            uuid::Uuid::new_v4(),
            FIRST_OBSERVATION_SEQUENCE,
        )
        .unwrap(),
        terminal: terminal.runtime.unwrap().target,
        provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
        working_directory: None,
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: u64::default(),
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap()
}
