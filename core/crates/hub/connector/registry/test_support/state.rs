use super::constants::*;
use crate::state::{Context, State};
use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_connector_protocol::*;
use yolo_hub_connector_store::Seed;
use yolo_hub_identity::*;

pub fn published() -> (Seed, State) {
    let hub = HubId::new();
    let seed = Seed {
        connector_id: ConnectorId::new(),
        request_id: Uuid::new_v4(),
        agent: AgentAddress {
            hub_id: hub,
            agent_id: AgentId::new(),
        },
        scope: ConnectorScope {
            terminal: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: hub,
                    terminal_id: TerminalId::new(),
                },
                runtime_epoch: RuntimeEpoch::new(),
            },
            provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
        },
        credential: ConnectorCredential::generate(),
    };
    let context = Context {
        instance: ConnectorInstanceId::new(),
        execution: ExecutionId::new(),
        generation: ConnectorGeneration::new(),
        stream: Uuid::new_v4(),
    };
    let target = AgentTarget {
        address: seed.agent,
        execution_id: context.execution,
    };
    let publication = Publication {
        connector_id: seed.connector_id,
        generation: context.generation,
        agent: target,
        terminal: seed.scope.terminal,
        sequence: FIRST_PUBLICATION_SEQUENCE,
        state: ReportedState {
            lifecycle: Lifecycle::Running {},
            activity: Activity::Working,
            working_directory: None,
        },
    };
    let snapshot = AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, context.stream, FIRST_PUBLICATION_SEQUENCE).unwrap(),
        terminal: seed.scope.terminal,
        provider: seed.scope.provider.clone(),
        working_directory: None,
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::Connector,
        observed_unix_ms: NOW,
        recovery: RecoveryStatus::Unsupported,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap();
    let mut state = State::initial();
    state.context = Some(context);
    state.last_sequence = FIRST_PUBLICATION_SEQUENCE;
    state.last_publication = Some(publication);
    state.snapshot = Some(snapshot);
    (seed, state)
}
pub fn snapshot(state: &mut State, mutate: impl FnOnce(&mut AgentSnapshotFields)) {
    let mut fields = state.snapshot.as_ref().unwrap().fields().clone();
    mutate(&mut fields);
    state.snapshot = Some(AgentSnapshot::new(fields).unwrap());
}
