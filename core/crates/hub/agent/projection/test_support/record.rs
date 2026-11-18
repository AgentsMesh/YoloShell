use crate::{
    constants::SCHEMA_VERSION,
    identity,
    record::{Entry, Record},
};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::*;
use yolo_terminal_model::TerminalView;

pub fn record(view: &TerminalView) -> Record {
    let hub = HubId::new();
    let binding = view.agent.as_ref().unwrap();
    let subject = identity::subject(binding).unwrap();
    let agent_id = AgentId::new();
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: hub,
            agent_id,
        },
        execution_id: ExecutionId::new(),
    };
    let runtime = RuntimeEpoch::from_uuid(view.runtime.as_ref().unwrap().runtime_epoch).unwrap();
    let snapshot = AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap(),
        terminal: RuntimeTarget {
            address: TerminalAddress {
                hub_id: hub,
                terminal_id: subject.terminal_id,
            },
            runtime_epoch: runtime,
        },
        provider: ObservedProvider::new(
            subject.kind.clone(),
            None,
            Some(subject.native_reference.clone()),
        )
        .unwrap(),
        working_directory: Some(AgentWorkingDirectory::new(binding.cwd.clone()).unwrap()),
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: binding.updated_at_ms as u64,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap();
    let entry = Entry {
        subject: subject.clone(),
        runtime,
        process: identity::process(binding),
        detected_at_ms: binding.detected_at_ms,
        updated_at_ms: binding.updated_at_ms,
        source_exited: false,
        control_witness: None,
        snapshot,
    };
    let root = tempfile::tempdir().unwrap();
    let directory = yolo_paths::directory::PrivateDirectory::ensure(root.path())
        .unwrap()
        .durable_identity()
        .unwrap();
    Record {
        schema: SCHEMA_VERSION,
        hub,
        directory,
        entries: BTreeMap::from([(agent_id, entry)]),
        latest: BTreeMap::from([(subject.terminal_id, agent_id)]),
        retired_controls: BTreeMap::new(),
    }
}
