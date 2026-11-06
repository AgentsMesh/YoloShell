use super::constants::*;
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::*;
use yolo_hub_rpc_protocol::HubSummary;
use yolo_hub_terminal_protocol::{
    RuntimeStatus, RuntimeSummary, TerminalSummary, TerminalSummaryFields,
};
use yolo_tui_inventory::{Inventory, TreeSelection};

pub struct Fixture {
    pub agent: AgentSnapshot,
    pub session: TerminalSummary,
    pub hub: HubSummary,
}

impl Fixture {
    pub fn new() -> Self {
        let hub = HubId::new();
        let address = AgentAddress {
            hub_id: hub,
            agent_id: AgentId::new(),
        };
        let target = AgentTarget {
            address,
            execution_id: ExecutionId::new(),
        };
        let terminal = RuntimeTarget {
            address: TerminalAddress {
                hub_id: hub,
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        };
        let agent = AgentSnapshot::new(AgentSnapshotFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE)
                .unwrap(),
            terminal,
            working_directory: Some(
                AgentWorkingDirectory::new(AGENT_WORKING_DIRECTORY.into()).unwrap(),
            ),
            provider: ObservedProvider::new(
                PROVIDER_KIND.into(),
                Some(PROVIDER_VERSION.into()),
                Some(NATIVE_REFERENCE.into()),
            )
            .unwrap(),
            lifecycle: Lifecycle::Running {},
            activity: Activity::Working,
            reachability: Reachability::Reachable,
            freshness: Freshness::Current,
            source: ObservationSource::ProviderHook,
            observed_unix_ms: OBSERVED_MS,
            recovery: RecoveryStatus::Available,
            capabilities: AgentCapabilities::UNSUPPORTED,
            interaction: None,
        })
        .unwrap();
        let session = TerminalSummary::new(TerminalSummaryFields {
            presentation: Default::default(),
            address: terminal.address,
            name: SESSION_NAME.into(),
            cwd: WORKING_DIRECTORY.into(),
            archived: false,
            recovery: None,
            runtime: Some(RuntimeSummary {
                target: terminal,
                status: RuntimeStatus::Running,
                output_sequence: 0,
                size: None,
                safe_termination: None,
            }),
            agent: Some(target),
            workspace: None,
        })
        .unwrap();
        Self {
            agent,
            session,
            hub: HubSummary {
                hub_id: hub,
                reachable: true,
                label: None,
            },
        }
    }

    pub fn terminals(&self) -> BTreeMap<TerminalAddress, TerminalSummary> {
        BTreeMap::from([(self.session.fields().address, self.session.clone())])
    }

    pub fn inventory(&self) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.replace(Some(vec![self.agent.clone()]), Some(vec![self.hub.clone()]));
        inventory.current = true;

        inventory.reconcile("", &self.terminals());
        inventory.select_tree(
            TreeSelection::Agent(self.agent.cursor().target()),
            "",
            &self.terminals(),
        );
        inventory
    }
}
